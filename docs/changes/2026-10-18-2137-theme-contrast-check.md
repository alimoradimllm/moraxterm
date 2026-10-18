# Measure text contrast in every theme and fix the Light primary button

New `scripts/ui_theme_contrast.py` loads the built UI in headless Edge under each application theme and computes the WCAG contrast ratio of every visible text element (resolving translucent and `color-mix()` backgrounds). It found one real failure: in Light, the white label on the teal "Start local terminal" button was 4.03:1, under the 4.5:1 minimum for 14px text. The Light accent is now `#087a66` (white on it is 5.27:1).

Amber and Midnight, reviewed by eye last time, measure clean: no text element below 4.5:1 in any of the five themes.

Verified: before the fix the script reported 1 failing element in Light and 0 in the other four; after rebuilding, 0 in all five (46 text elements each). The script's first run also showed a false "1.3:1" on the active toolbar label because it misread `color(srgb ...)` values; I fixed the parser and checked the label by eye, where it is readable.

Open: this measures text only. Icon-on-tile contrast, borders and focus rings are judged by eye, not measured. Light with real terminal output and the exe under each theme were not checked.
