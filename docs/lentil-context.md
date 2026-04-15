# Lentil Context

## Project Summary

- `lentil` is a fast capture and task visualization application built as a wrapper over Taskwarrior.
- Taskwarrior remains the source of truth for storage and core productivity logic.
- Lentil is meant to add:
- Fast capture
- Opinionated visualization
- Rule-based understanding of the user's existing task system
- The first target platform is Linux desktop.
- macOS and Windows should still be kept in mind architecturally.
- The project is intended to remain GPL 3.x.

## Product Intent

- The capture experience is the primary focus of the MVP.
- Capture should feel seamless, minimal, distraction-free, and aesthetically sharp.
- Visualization should exist separately and must not clutter capture by default.
- Lentil is being dogfooded during development, so real usage feedback is being collected live through Taskwarrior tasks created from Lentil itself.

## MVP Scope

- Fast capture desktop window.
- Taskwarrior-backed task creation.
- Fields exposed in MVP:
- Task title
- Project
- Due date
- Priority
- Freeform tags
- A basic read-only task view later, in a separate window, is acceptable for MVP.

## Confirmed UX Requirements

- The capture UI should feel like one dialog, not multiple screens.
- The title stays visible while metadata entry appears underneath it.
- Metadata guidance order is:
- Project
- Due
- Priority
- Tags
- `Ctrl+D` is the explicit continue / submit key.
- `Esc` is the explicit backward-navigation key.
- `Tab` should not advance the capture state machine.
- At the current stage of development, `Tab` is intentionally suppressed rather than used for navigation.
- The due-date UI should be keyboard-friendly and use a custom aesthetic calendar rather than a native date picker.
- Due dates are optional.
- If a due date is present, priority is mandatory.
- The default post-submit behavior is clear and close.
- A configurable clear-and-stay-open behavior should exist.
- Capture should be summoned by a dedicated global hotkey eventually.
- The read-only / dashboard / visualization experience must stay separate from capture by default.

## Current Capture Behavior

As of 2026-04-15, the capture implementation in [src/components/hero.rs](/home/kakkar/Desktop/projects/OSS/lentil/src/components/hero.rs) behaves like this:

- The desktop window is always-on-top, undecorated, non-resizable, and opaque.
- Transparency was removed temporarily because it caused ghosting on Fedora 43.
- The title field is shown first.
- Pressing `Ctrl+D` on the title enters metadata mode and reveals only the next guided section.
- The capture flow is modeled using Rust enums:
- `CaptureState`
- `DetailStep`
- Project, due, priority, and tags are revealed one step at a time based on the current capture state.
- `Esc` moves backward through the capture state machine.
- `Ctrl+D` advances through the guided steps and submits on the tags step.
- Mouse focus should also move the capture state machine backward when the user clicks into an earlier field, including the title field.
- `Tab` is currently suppressed at the shell level and also via JS capture-phase handling in the webview so it does not interfere with the interaction model.
- `Shift+Tab` suppression is being hardened using capture-phase interception on both `keydown` and `keyup` at `window` and `document` level because the desktop/webview stack has been inconsistent here.

## Current Due-Date Rules

- The due field stores a display-form date as `dd/mm/YYYY`.
- On Taskwarrior submission, that date is converted to `YYYY-MM-DD`.
- Due input currently accepts:
- `dd`
- `dd/mm`
- `/mm`
- `dd/mm/YYYY`
- `/mm/YYYY`
- Current partial-date rules:
- `dd/mm` defaults the year to the current year.
- `/mm` defaults the day to today's day and the year to the current year.
- `/mm/YYYY` defaults the day to today's day.
- Single-segment input currently has special handling:
- If the number is greater than `12`, it is treated as a day in the current month/year.
- If the number is `12` or less, it is currently treated as a month with today's day and current year.
- This behavior was introduced to support "current day as default" semantics.
- The due date is optional overall.
- When entering the due step, today's date is currently kept as an implicit default while the text field can remain visually empty.
- The preview text should communicate this implicit default clearly.
- If the due field is still empty while using that implicit default, the first arrow-key action should commit today's date before any later arrow-key shifts move away from it.

## Current Suggestion Behavior

- Project suggestions are loaded from `task _projects`.
- Tag suggestions are loaded from `task _tags`.
- Suggestions are ranked using a simple mix of:
- Query match
- Title relevance
- Frequency-like ordering from the loaded list
- The `idea` tag can be suggested implicitly when no due date exists, controlled by config.
- Project suggestions are keyboard-selectable with arrow keys and accepted with `Enter`.
- Tag suggestions are keyboard-selectable with arrow keys and inserted with `Enter`.

## Current Config Scaffold

Config lives at:

- `~/.config/lentil/config.toml`

Current first-pass config keys in [src/config.rs](/home/kakkar/Desktop/projects/OSS/lentil/src/config.rs):

- `clear_and_stay_open`
- `suggest_idea_without_due`
- `require_priority_with_due`

Current defaults:

- `clear_and_stay_open = false`
- `suggest_idea_without_due = true`
- `require_priority_with_due = true`

These are intentionally simple and should be extended later.

## Known Product Decisions

- Lentil should only write standard Taskwarrior fields in MVP, plus normal freeform tags.
- User-specific workflow rules should live in Lentil config, not be buried forever in hardcoded behavior.
- The "due date requires priority" rule can be hardcoded initially but should become configurable later.
- Behaviors like suggesting the `idea` tag when there is no due date should also become configurable.

## Implementation Notes

- The app entrypoint is in [src/main.rs](/home/kakkar/Desktop/projects/OSS/lentil/src/main.rs).
- The main capture component is in [src/components/hero.rs](/home/kakkar/Desktop/projects/OSS/lentil/src/components/hero.rs).
- Styling is in [assets/styling/main.css](/home/kakkar/Desktop/projects/OSS/lentil/assets/styling/main.css).
- The current implementation uses shelling out to Linux `date` for date validation and date shifting.
- This is acceptable for Linux-first iteration but should be replaced or abstracted before serious cross-platform work.
- The current implementation shells out to `task` for Taskwarrior integration.

## Known Issues / Follow-Up

- Window transparency is currently disabled because of Fedora 43 ghosting.
- If transparency is revisited, it needs a compositor-safe solution rather than reusing the old flags.
- The capture interaction has gone through several iterations around `Tab` and `Shift+Tab`.
- `Shift+Tab` proved unreliable in the current webview / desktop event stack and is not part of the intended current interaction model.
- The title textarea auto-sizing and footer spacing have required repeated tuning and are still sensitive areas.
- The title textarea now has a larger vertical growth budget, explicit hidden vertical overflow, and extra measurement/padding buffer to avoid clipping descenders like `g` and `y`, but it remains a sensitive area that should be rechecked in live usage.
- Due-date defaults and partial-date semantics are another sensitive area and should be rechecked in live usage.

## Working Development Process

- The user is dogfooding Lentil while building it.
- The user records iterative implementation feedback into Taskwarrior, often through Lentil itself.
- On returning to the prompt in this repository, the default workflow should be:
- Check Taskwarrior first
- Filter by `project:lentil`
- Treat that as the active feedback queue
- The exact command to use by default is:
- `task project:lentil status:pending`

## Current Open Questions

- What exact default global hotkey should Lentil use on Linux for fast capture?
- When the read-only task view is implemented, what is the preferred window behavior and invocation hotkey?
- How should rule-based relevance for suggestions evolve beyond the current simple ranking logic?
- When should visualization work begin relative to more capture iteration?

## Immediate Next-Step Context

If work resumes from here, the most likely next actions are:

- Read `task project:lentil status:pending`
- Fix the next dogfooding issues in the capture flow
- Keep the context file up to date as capture semantics change
