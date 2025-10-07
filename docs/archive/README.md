# Archive

**Last Updated**: October 7, 2025

## Purpose

This directory contains historical documentation that is no longer actively maintained but preserved for reference. These documents capture implementation details, bug fixes, and completion summaries from the project's development history.

## Structure

### `fixes/`
Bug fix documentation and post-mortems from various issues encountered during development.

**Contents:**
- `BUFFER_SCROLLING_FIX.md` - Fix for buffer scrolling issues
- `COLON_KEYBINDING_FIX.md` - Fix for colon keybinding conflicts
- `OPML_AUTOCOMPLETE_FIX.md` - Fix for OPML autocomplete functionality
- `OPML_REAL_WORLD_FIX_SUMMARY.md` - Summary of real-world OPML handling fixes
- `OPML_XML_SANITIZATION_FIX.md` - XML sanitization improvements for OPML
- `TESTING_OPML_URL_FIX.md` - Test documentation for OPML URL handling
- `BUGFIX_OPML_URL_HANDLING.md` - Detailed bug fix for OPML URL handling

### `implementation_notes/`
Detailed notes from feature implementations and technical decisions made during development.

**Contents:**
- `OPML_IMPROVEMENTS.md` - Improvements made to OPML support
- `WHATS_NEW_BUFFER_IMPLEMENTATION.md` - Details about buffer-based UI implementation
- `OPML_IMPLEMENTATION_SUMMARY.md` - Summary of OPML feature implementation
- `FEATURE-OPML.md` - Original OPML feature planning and notes

### `summaries/`
Completion summaries and milestone documentation.

**Contents:**
- `OPML_BUG_FIX_COMPLETE.md` - OPML bug fix completion summary
- `OPML_FEATURE_COMPLETE.md` - OPML feature completion announcement
- `IMPLEMENTATION_COMPLETE.md` - General implementation completion notes
- `ISSUE_FIXES_SUMMARY.md` - Summary of various issue fixes
- `DOCUMENTATION_UPDATE_SUMMARY.md` - Documentation update summary
- `SETUP_COMPLETE.md` - Build system setup completion
- `WINDOWS_BUILD_COMPLETE.md` - Windows build system completion

## Why Archive?

These documents were moved to the archive for the following reasons:

1. **Historical Context**: Valuable for understanding past decisions but not needed for current development
2. **Reduced Clutter**: Keeping the root and `docs/` directories focused on active, user-facing documentation
3. **Maintainability**: Easier to navigate the project when historical documents are separated
4. **Git History**: All changes are still preserved in git history; these documents provide narrative context

## Current Documentation

For current, active documentation, see:

- **Root Directory**: User-facing documentation (README, GETTING_STARTED, CHANGELOG, CONTRIBUTING)
- **docs/**: Technical documentation (PRD, ARCHITECTURE, BUILD_SYSTEM, KEYBINDINGS, etc.)
- **scripts/README.md**: Build script documentation

## Note

If you need to reference any of these documents, they remain valid historical records. However, always check the current documentation first as processes and implementations may have evolved since these were written.
