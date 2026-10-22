//! the two read-only pages the kernel exposes to every process.

use zakuro_common::memory_map::{CONFIG_MEM_SIZE, SHARED_PAGE_SIZE};
use zakuro_common::ConsoleModel;

use crate::kernel::thread::{CPU_CLOCK_HZ, ticks_to_nanos};

/// seconds between the 3DS epoch (1900-01-01) and the Unix epoch.
const EPOCH_OFFSET_SECONDS: u64 = 2_208_988_800;

/// a plausible MAC. Games only ever show it or hash it.
pub const MAC_ADDRESS: [u8; 6] = [0x40, 0xF4, 0x07, 0x00, 0x00, 0x01];

fn write_u32(page: &mut [u8], offset: usize, value: u32) {
    page[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u64(page: &mut [u8], offset: usize, value: u64) {
    page[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

/// fills the configuration page.
pub fn init_config_mem(
    page: &mut [u8],
    model: ConsoleModel,
    app_mem_type: u32,
    app: u32,
    sys: u32,
    base: u32,
) {
    assert_eq!(page.len(), CONFIG_MEM_SIZE as usize);
    page.fill(0);

    // report firmware 11.x, which is what every retail console ended on.
    page[0x00] = 0x00; // kernel version revision
    page[0x01] = 0x34; // kernel version minor
    page[0x02] = 0x02; // kernel version major
    write_u32(page, 0x04, 0); // update flag
    write_u64(page, 0x08, 0); // NS title id
    write_u32(page, 0x10, 0x0000_0002); // syscore version
    page[0x14] = 0x01; // env info, production
    // unit info bit 0 distinguishes retail from a development unit.
    page[0x15] = 0x01;
    page[0x16] = 0x00; // previous firm
    write_u32(page, 0x18, 0x0000_F297); // CTR SDK version

    write_u32(page, 0x30, app_mem_type);
    write_u32(page, 0x40, app);
    write_u32(page, 0x44, sys);
    write_u32(page, 0x48, base);

    page[0x60] = 0x00;
    page[0x61] = 0x34;
    page[0x62] = 0x02;
    write_u32(page, 0x64, 0x0000_0002);
    write_u32(page, 0x68, 0x0000_F297);

    let _ = model;
}

/// live state.
pub fn init_shared_page(page: &mut [u8], model: ConsoleModel, slider_3d: f32, clock: u64) {
    assert_eq!(page.len(), SHARED_PAGE_SIZE as usize);
    page.fill(0);

    write_u32(page, 0x00, 0); // date/time selector, slot 0 is current
    page[0x04] = 1; // running hardware, retail product
    page[0x05] = if model.is_new3ds() { 2 } else { 1 };

    update_datetime(page, clock, 0);

    page[0x60..0x66].copy_from_slice(&MAC_ADDRESS);
    page[0x66] = 3; // full wifi signal
    page[0x67] = 2; // wifi enabled and connected

    set_slider_3d(page, slider_3d);
    // battery, on the adapter, full and no longer charging
    page[0x85] = 1 | 5 << 2;
    page[0xC0] = 0; // no headset
}

/// the host's local time, in milliseconds since 1900.
pub fn host_clock() -> u64 {
    let unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    // the console keeps the local time, not UTC
    let local = chrono::Local::now().offset().local_minus_utc() as i64 * 1000;
    (unix as i64 + local) as u64 + EPOCH_OFFSET_SECONDS * 1000
}

/// the console's clock, in milliseconds since 1900, the clock at boot
/// moved on by the emulated ticks since.
pub fn clock_at(boot_clock: u64, tick: u64) -> u64 {
    boot_clock + ticks_to_nanos(tick) / 1_000_000
}

/// a time of the console's clock as people read it, "Wed 21 Oct 2026, 20:31".
pub fn clock_text(clock: u64) -> String {
    match local_time(clock) {
        Some(time) => time.format("%a %-d %b %Y, %H:%M").to_string(),
        None => "an unreadable time".to_owned(),
    }
}

/// how many milliseconds from a time of the console's clock until the hour
/// given next starts, the next day's when it has started today.
pub fn until_hour(clock: u64, hour: u32) -> u64 {
    let Some(time) = local_time(clock) else { return 0 };
    let Some(today) = time.date().and_hms_opt(hour.min(23), 0, 0) else { return 0 };
    let next = if today > time { today } else { today + chrono::Duration::days(1) };
    (next - time).num_milliseconds().max(0) as u64
}

/// a time of the console's clock, which keeps the local time, as a date and
/// time with no time zone.
fn local_time(clock: u64) -> Option<chrono::NaiveDateTime> {
    let unix = clock as i64 - EPOCH_OFFSET_SECONDS as i64 * 1000;
    Some(chrono::DateTime::from_timestamp_millis(unix)?.naive_utc())
}

/// refreshes the clock fields, the clock at boot moved on by the emulated
/// ticks since. titles extrapolate from it with the tick, so a clock
/// following the host's would run backwards whenever emulation ran ahead.
pub fn update_datetime(page: &mut [u8], boot_clock: u64, tick: u64) {
    let date_time = clock_at(boot_clock, tick);

    for slot in [0x20usize, 0x40] {
        write_u64(page, slot, date_time);
        write_u64(page, slot + 0x08, tick);
        // the rate of the ticks, which the SDK divides the ticks since the
        // update by to get the milliseconds since, as PTM sets it. anything
        // else makes the clock jump ahead within a frame and back at the
        // next update, and a title that keeps a daily limit sees its clock
        // set back
        write_u32(page, slot + 0x10, CPU_CLOCK_HZ as u32);
        write_u32(page, slot + 0x14, 0);
        write_u64(page, slot + 0x18, 0);
    }
}

/// updates the 3D slider position, which games poll every frame. the byte
/// after it is set when the 3D LED is forced off, GSPGPU:SetLedForceOff,
/// which games take as the 3D being off, so it stays clear.
pub fn set_slider_3d(page: &mut [u8], value: f32) {
    page[0x80..0x84].copy_from_slice(&value.clamp(0.0, 1.0).to_le_bytes());
    page[0x84] = 0;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// the time a title reads, the way the SDK works it out of the page,
    /// its selected slot's time plus the ticks since its update times 1000
    /// over the rate in the slot.
    fn sdk_time(page: &[u8], tick: u64) -> u64 {
        let slot = 0x20;
        let read = |at: usize, size: usize| page[at..at + size].iter().rev().fold(0u64, |v, b| v << 8 | *b as u64);
        let (date_time, updated, rate) = (read(slot, 8), read(slot + 8, 8), read(slot + 0x10, 4) as i32 as i64);
        let per_tick = (1000i64 << 32) / rate;
        date_time + (((tick - updated) as i128 * per_tick as i128) >> 32) as u64
    }

    /// a title reads a clock that only ever goes forward, at the same pace
    /// as the ticks, between the updates once a frame and across them.
    #[test]
    fn the_clock_titles_read_runs_at_the_ticks_pace() {
        let mut page = vec![0u8; SHARED_PAGE_SIZE as usize];
        let boot = 3_900_000_000_000;
        let frame = crate::CYCLES_PER_FRAME;
        update_datetime(&mut page, boot, 0);
        let mut last = sdk_time(&page, 0);
        for step in 1..=600u64 {
            let tick = step * frame / 4;
            if step % 4 == 0 {
                update_datetime(&mut page, boot, tick);
            }
            let now = sdk_time(&page, tick);
            assert!(now >= last, "the clock went back from {last} to {now}");
            last = now;
        }
        // 150 frames, about 2.5 s
        let elapsed = last - boot;
        assert!((2500..=2510).contains(&elapsed), "{elapsed} ms");
    }

    /// a time reads as the date and time it is, and the next start of an
    /// hour is today's when it is still to come and tomorrow's when not.
    #[test]
    fn times_read_as_people_read_them() {
        // 21 October 2026, 20:31:15, since 1900
        let unix = chrono::NaiveDate::from_ymd_opt(2026, 10, 21).unwrap().and_hms_opt(20, 31, 15).unwrap().and_utc().timestamp_millis();
        let clock = (unix + EPOCH_OFFSET_SECONDS as i64 * 1000) as u64;
        assert_eq!(clock_text(clock), "Wed 21 Oct 2026, 20:31");
        let minute = 60_000;
        assert_eq!(until_hour(clock, 23), (2 * 60 + 28) * minute + 45_000);
        assert_eq!(until_hour(clock, 7), (10 * 60 + 28) * minute + 45_000);
        assert_eq!(clock_text(clock + until_hour(clock, 7)), "Thu 22 Oct 2026, 07:00");
    }

    /// the clock starts at the local time, which is what the console shows.
    #[test]
    fn the_clock_is_local() {
        let utc = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;
        let offset = chrono::Local::now().offset().local_minus_utc() as i64 * 1000;
        let clock = host_clock() as i64 - EPOCH_OFFSET_SECONDS as i64 * 1000;
        assert!((clock - utc - offset).abs() < 1000);
    }
}
