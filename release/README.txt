AppSweep 1.0.0 - Windows app manager and deep uninstaller
==========================================================

Run
---
Run AppSweep.exe directly. No installation is needed.
A full purge asks for administrator rights (UAC).

Features
--------
- Lists every installed app: desktop programs, Store apps and shortcuts.
- Includes system apps and shared runtimes (hide them in Settings).
- Card or table view, with the icon of each app.
- Instant search and filters by type, favorites, and "Remnant" (entries whose files are gone).
- Every app has a stable numeric id shown on the card, in the table and in the details sheet.
  Search by the number directly (example: 449 or #449).
- Three buttons per app:
  * Uninstall: the official uninstaller, then silent, then MSI, then the package
               manager, trying the other methods automatically.
  * Files    : delete only the app's files.
  * Purge    : delete files, registry, shortcuts, services, scheduled tasks and
               leftovers, with a registry backup and a restore point.
- Full details sheet: publisher, version, size, install date, real architecture
  (read from the executable header), paths, registry key, detected traces and your own notes.
- Backup manager: browse the saved registry backups, restore them or delete them.
- Final confirmation requires the numeric id (not the name), because some names
  contain glyphs that cannot be typed. The button stays disabled until it matches.
- Built-in protection: system and drive-root folders can never be deleted, and the
  buttons stay dimmed for those apps.

Settings
--------
Saved in: %APPDATA%\AppSweep\settings.json
Registry backups: %APPDATA%\AppSweep\backups

Command line
------------
AppSweep.exe --version     show the version
AppSweep.exe --selftest    safe self-test of the delete engine (temp folder only)
AppSweep.exe --dump        print the detected app list
AppSweep.exe --tasks PATH  show the scheduled tasks tied to a path
AppSweep.exe --plan FILE   run a delete plan (used internally after UAC)

Note
----
A purge cannot be undone. Check the path in the details sheet before confirming.

UI icons: MingCute (Apache-2.0) - https://www.mingcute.com
Brand marks for apps with no icon on disk: Simple Icons (CC0) - https://simpleicons.org
