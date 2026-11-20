# MLBB Heroes v2.7.0 build report

The owner confirmed the 1.1.164 Windows candidate after testing the in-match ban/pick hover fix. The public v2.7.0 install ZIP uses that exact Windows DLL and the tested 26-hero roster, audio, visuals and gameplay data. The release packaging updates the README and screenshot guide.

The candidate build passed 552 Rust tests, with one existing ignored test, 104 champion-data checks, four loader tests and 192 Atlas view bindings. The final Windows ZIP has a single `mlbb_heroes/` folder. Both final ZIPs passed CRC checks, and all 1,551 install entries matched their full-source counterparts. A file-by-file comparison with the owner-tested candidate found differences only in the README and screenshot guide. The source archive includes the frozen SDK and the 1.1.164 validation reports.

The owner confirmation covers the reported hover issue. The project has no live FPS measurement for this change. [Owner-tested build record](https://app.notion.com/p/41e9c01d37c9810ead0ff6650bab284d)
