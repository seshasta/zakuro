Tomodachi Life (PC), test version
=================================

Tomodachi Life (US) running natively on Windows, from your own copy of the
game. Nothing from Nintendo comes with this download: you add your own game
dump and a few files from your own 3DS the first time it starts.

What you need
  - Windows 10 or 11, 64-bit, and a graphics card with Vulkan or OpenGL
    (any recent Intel, AMD or NVIDIA one).
  - A decrypted dump of Tomodachi Life, US version (000400000008C300),
    from your own cartridge or eShop copy.
  - From your own 3DS, with GodMode9: the system font and the Mii
    resources. The keys file is optional (Mii QR codes only).
  - About 2 GB free, plus 400 MB once for a compiler, which setup offers to
    download if this PC has none.
  - On a Windows "N" edition, the Media Feature Pack from Microsoft.

First run
  1. Unzip this folder anywhere you like (not inside Program Files).
  2. Start "Tomodachi Life.exe". It opens on a setup screen.
     If Windows says "Windows protected your PC", choose "More info", then
     "Run anyway": the program isn't signed, as test versions usually aren't.
  3. Add each file: press "Choose..." or drag the file onto the window.
     Each one is checked and copied into this folder. "How to get it" under
     each one says where to find it on your 3DS with GodMode9. For the font,
     use cbf_std.bcfnt.lz; it's converted for you.
  4. Press "Recompile". This turns the game's code into code for this PC so
     it runs at full speed. It takes a while, once. You can press Start and
     play meanwhile; it switches over when it's done. Without it the game
     still runs, but slower, and the Miis' voices may stutter.
     If this PC has no C compiler, setup offers to download Zig, one that
     needs no installing. Already downloaded Zig yourself? It has to be
     0.17.0: press "Use a Zig I downloaded..." and choose its zip (or
     zig.exe, if you unzipped it), or drag the zip onto the window.
  5. Press "Start Tomodachi Life". From then on it starts straight into the
     game.

Controls
  Mouse              touch screen (click and drag)
  X / Z              A / B        S / A    X / Y
  Arrow keys         D-pad        Enter    Start
  Esc                menu (settings, layout, controls, clock, quit)
  F11                full screen
  A controller works too; set its buttons in Esc > Settings.

Clock
  Esc > Clock sets the game's clock on: to the next morning, noon, evening
  or night, or by an hour, six hours or a day. The game sees the new time
  within a moment and keeps it when you start it again. "Back to the
  computer's time" undoes it.

Confessions, proposals and weddings, on demand
  1. Save in the game and close it (Esc > Quit).
  2. Double-click "Set up a romance.bat". It lists your islanders with a
     number each, then asks:
       1  a confession: one islander confesses to another, or
       2  a proposal: makes the two sweethearts, and one proposes.
     Type the numbers it asks for. It only works for a man and a woman who
     are both single (the US game has nothing else), and it says so if not.
  3. Start the game, during the day, and go to the Apartments. The one who
     confesses or proposes has a pink heart on their window. Go in and tap
     their thought bubble.
  If there's no heart: the islander may be asleep or out visiting. Use
  Esc > Clock (next morning, or an hour on) and look again.
  Each run keeps the save as it was in "save backups\<date and time>\". To
  undo, close the game and copy that savedataArc.txt back over
  user\savedata\000400000008C300\savedataArc.txt.

Mii QR codes
  Needs the keys file. Drag a picture of a QR code onto the window, then scan
  as usual in the game. The cameras keep seeing that picture until you drop
  another one or change the camera in Esc > Settings.

Reporting a problem
  1. Start the game with "Record a bug.bat" instead of the .exe, and play
     until the problem happens. Then close the game.
  2. Notepad opens notes.txt: write what happened, roughly when, and what
     you expected.
  3. Zip the newest folder in recordings\ and send it, with a screenshot if
     you have one. It holds your inputs, your save as it was when you
     started, your settings and the log, so the problem can be replayed
     exactly. It holds nothing from the game itself.
  If the game won't start at all, send zakuro.log from this folder.

What's in this folder
  Tomodachi Life.exe   the game, on the Zakuro 3DS emulator
  app.toml             which game this folder runs
  settings.toml        your settings
  Record a bug.bat     plays as usual, recording what happens (see above)
  Set up a romance.bat sets up a confession or a proposal (see above)
  tools\               what "Set up a romance.bat" runs
  LICENSE (Zakuro).txt, THIRD-PARTY-NOTICES.txt   the licences
  And once set up:
  game\                your game dump
  sysdata\             the files from your 3DS
  recompiled\          the game's code, recompiled for this PC
  user\                your saves

Keep it private
  game\, sysdata\ and recompiled\ come from Nintendo's game and system and
  from your own console. Don't share them or upload them anywhere.

This runs on Zakuro, an MIT-licensed 3DS emulator (see LICENSE (Zakuro).txt),
with changes from github.com/seshasta/zakuro, and on code 3dsrecomp
recompiles on this PC. The other software it's built from is listed in
THIRD-PARTY-NOTICES.txt.
