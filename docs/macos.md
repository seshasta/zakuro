# Zakuro on a Mac (Apple Silicon)

Where a Mac build of Zakuro stands, and what is left before Tomodachi Life
can go to Mac players. Written from a Windows PC with no Mac at hand.
Builds and tests come from GitHub Actions' `macos-latest` runner, an M1 in
a virtual machine.

## What already works

Most of Zakuro was written with a Mac in mind:

- **Finding folders:** settings, saves and caches go to
  `~/Library/Application Support/zakuro` and `~/Library/Caches/zakuro`.
- **Fonts:** Japanese, Korean and Chinese fonts come from
  `/System/Library/Fonts`.
- **Camera:** the camera library uses AVFoundation.
- **Zig:** Zakuro downloads `zig-aarch64-macos` when there is no C compiler.
- **Recompiler:** 3dsrecomp builds a `.dylib` with the Mac's `cc`, which is
  clang from Xcode's Command Line Tools, or with Zig. Zakuro loads it with
  `dlopen`. The C it writes has no x86-only code, and the guest's memory
  goes through page tables in software rather than the host's own pages,
  so Apple's 16 KB pages don't matter.
- **Without Vulkan:** the window falls back to OpenGL, which a Mac still has
  up to version 4.1, and the 3DS's GPU is drawn on the CPU.

`cargo check --target aarch64-apple-darwin -p zakuro -p zakuro-core -p
zakuro-gpu --tests` passes from Windows, with a stand-in C compiler for
the one Objective-C file in the camera library's dependencies. It found no
errors in Zakuro's code. Linking, and anything that only shows when the
program runs, is left for the CI job.

## What this branch changes

- **CI:** `ci.yml` has a `macos` job. It installs MoltenVK and the Vulkan
  loader from Homebrew, builds `-p zakuro`, and runs the zakuro,
  zakuro-core and zakuro-gpu tests. It then runs one more test,
  `the_renderer_finds_a_gpu`, which says whether the renderer found a GPU
  and, if not, what the GPU lacks. That test is allowed to fail. The job
  keeps the Mac program as an artifact.
- **Loading Vulkan:** `load_vulkan()` tries the system's `libvulkan.dylib`
  first, then `libvulkan.1.dylib` or `libMoltenVK.dylib` in the app's
  `Contents/Frameworks`, next to the program, and in Homebrew's `lib`
  folders. On a Mac, `dlopen` doesn't look in `/opt/homebrew/lib` by
  default.
- **Instance and device:** on a Mac, the instance asks for
  `VK_KHR_portability_enumeration` and sets `ENUMERATE_PORTABILITY_KHR`.
  Without these, the Vulkan loader hides MoltenVK, because MoltenVK
  doesn't implement all of Vulkan. Devices are made with
  `VK_KHR_portability_subset` when the GPU offers it, which the Vulkan spec
  requires. Windows and Linux don't ask for any of this, so they behave as
  before.
- **Error message:** when no GPU qualifies, the error now names each GPU
  and what it lacks, instead of only listing the requirements.

## The GPU renderer on a Mac

The renderer in `raster/hardware.rs` needs Vulkan 1.3, push descriptors,
depth clamping and the depth format `D24_UNORM_S8_UINT` (24-bit depth and
8-bit stencil, like the 3DS).

- **Vulkan 1.3:** MoltenVK has offered this since its 1.3 release in May
  2025.
- **Push descriptors and depth clamping:** MoltenVK has both.
- **Logic ops:** Metal doesn't have them, but the renderer already handles
  a GPU without them.
- **D24S8:** this is the blocker. Apple's own GPUs, so every Apple Silicon
  Mac, have no 24-bit depth format. Metal on them offers
  `Depth32Float_Stencil8` instead. The renderer then turns the GPU down,
  and the game draws on the CPU.

The fix is to choose the depth format at run time: D24S8 where the GPU has
it, and `D32_SFLOAT_S8_UINT` otherwise. Every place that copies depth
between an image and a buffer treats it as a 24-bit integer in a 32-bit
word. There are about six such places in `hardware.rs`, plus
`depth.comp` and the depth blits. With D32 those would convert to and from
a float (`d24 / 0xFFFFFF`). Depth bias needs a look too, because Vulkan
scales it differently for float formats. Most Windows GPUs also have
D32S8, so all of this can be built and tested on this PC first, by forcing
the float format with an environment variable.

### What the first CI run showed

[Run 40559165033](https://github.com/seshasta/zakuro/actions/runs/40559165033),
on commit 146d5b1:

- **Build and tests:** the program builds and links on the runner, an M1
  under macOS. The tests pass: 46 in zakuro, 147 in zakuro-core and 175 in
  zakuro-gpu, so the Cranelift JIT's shader tests pass on arm64.
- **GPU:** the runner does have one. `vulkaninfo` shows "Apple Paravirtual
  device" through MoltenVK 1.4.2, with Vulkan 1.4 and
  `VK_KHR_portability_enumeration`.
- **Why the renderer turns it down:**
  `the_renderer_finds_a_gpu` fails with "Apple Paravirtual device has no …
  D24S8 depth format", and nothing else is missing. So D24S8 is the only
  blocker. Once the D32 path exists, the GPU tests can run for real in CI,
  on the virtual GPU.
- **Until then:** the GPU tests on the Mac pass by skipping, since each
  one returns early when it can't get a device.

### Shader JIT (Cranelift)

Cranelift supports arm64 Macs, and `cranelift-native` picks Apple's
calling convention by itself. `cranelift-jit` writes code into memory,
then switches that memory to readable and executable with `mprotect`. It
doesn't use `MAP_JIT`. Apple Silicon allows this for programs without the
hardened runtime, which covers anything not notarized. The shader tests in
zakuro-gpu run the JIT, so the CI job checks it. Recommendation: **leave
it on.** If the program is ever notarized, it needs the
`com.apple.security.cs.allow-unsigned-executable-memory` entitlement,
because `allow-jit` only covers `MAP_JIT`.

## Packaging for Mac players

The Windows folder keeps everything next to the program: `app.toml`,
settings, saves, `sysdata/` and `recompiled/`. A `.app` can't do this.
macOS runs a downloaded app from a random read-only copy
(App Translocation) until the player moves it. Writing inside the bundle
would also break its signature. The proposed split:

```
Tomodachi Life.app/Contents/
  Info.plist            bundle id, NSCameraUsageDescription, icon
  MacOS/Tomodachi Life  zakuro
  Frameworks/libMoltenVK.dylib
  Resources/app.toml, README, icon.icns

~/Library/Application Support/Tomodachi Life/
  settings.toml, saves, sysdata/, recompiled/, game/, tools/zig-…
```

Code changes:

- **`app::load`:** read `app.toml` from `../Resources` when the program is
  inside an `.app`. Give `App` a read-only folder, where the manifest is,
  and a data folder, which is `~/Library/Application Support/<name>`
  inside a bundle and the program's own folder everywhere else. `dir`,
  `recompiled()`, `setup.rs`'s `sysdata/` paths, `settings::path()` and
  the `set_current_dir` in `lib.rs` all switch to the data folder.
  "Show folder" in the menu then opens it in Finder.
- **Camera:** `Info.plist` must have `NSCameraUsageDescription`. Without
  it, macOS kills the app the moment it opens the camera.
- **Windows-only tools:** the `.bat` and `.ps1` tools in
  `apps/tomodachi-life` don't run on a Mac. They become menu items, or
  `.command` shell scripts.
- **Packaging script:** a `package.sh` next to `package.ps1`, run by a
  macOS job in `release.yml`. It builds the bundle, copies in MoltenVK
  (Homebrew's, or the Vulkan SDK's), signs it and zips it, or makes a
  `.dmg`.

Recompiling on the player's Mac needs a C compiler. `xcode-select
--install` gives one, or Zakuro downloads Zig. Zig downloaded with curl
isn't quarantined, so Gatekeeper doesn't stop it.

### Signing and notarization

- **Unsigned (free):** Apple Silicon only runs code that is signed. Rust's
  linker signs ad hoc automatically, and the packaging script must sign
  the finished bundle again (`codesign --force --deep -s -`). Players then
  see "Apple could not verify…". Since macOS 15, Control-click → Open no
  longer gets past this. They have to try opening the app once, then
  click **Open Anyway** in System Settings → Privacy & Security, or run
  `xattr -dr com.apple.quarantine "Tomodachi Life.app"`. That's fine for
  testers, but rough for anyone else.
- **Developer ID and notarized ($99 a year):** this needs the Apple
  Developer Program, which publishes the signer's legal name. The app is
  signed with the hardened runtime and these entitlements:
  `allow-unsigned-executable-memory` for the JIT,
  `disable-library-validation` to load the `.dylib` recompiled on the
  player's Mac, and `device.camera`. Then `xcrun notarytool submit
  --wait` and `xcrun stapler staple`. All of it can run in GitHub Actions
  with the certificate and an App Store Connect API key stored as
  secrets.

## Plan and effort

| Step | What | Effort |
|---|---|---|
| 1 | Get the CI job green: fix whatever the real Mac linker or the tests turn up | ½–1 day |
| 2 | D32S8 depth when D24S8 is missing, tested on Windows by forcing it | 2–4 days |
| 3 | First run on a real Mac by a tester: MoltenVK gaps, window, audio, camera, controllers | 1–3 days, spread over a few rounds with the tester |
| 4 | `.app` bundle, data in Application Support, `package.sh`, release job | 1–2 days |
| 5 | Mac versions of the bug-recording and romance tools | ½–1 day |
| 6 | Developer ID signing and notarization (optional) | ½–1 day, plus $99 a year |

Recommended order: 1 → 4 → 3 with an unsigned build and CPU drawing, so a
Mac tester can try the game early. Then 2 for speed, then 5, and 6 only
once it goes beyond testers. Without step 2, Tomodachi Life still runs,
but the 3DS's GPU is drawn on the CPU, at the console's own resolution
only. How fast that is on an M-series chip hasn't been measured.
