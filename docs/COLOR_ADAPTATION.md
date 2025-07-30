# Color Adaptation for Terminal Backgrounds

## Overview

The syncable-cli tool now includes intelligent color adaptation that automatically adjusts colors based on your terminal's background theme, ensuring optimal readability whether you're using a dark or light terminal theme.

## Problem Statement

Different terminal applications and themes use varying background colors:
- **Dark terminals**: Black or dark backgrounds (most common)
- **Light terminals**: White or light backgrounds (less common but used)

Previously, the CLI used colors optimized for dark backgrounds, which could be hard to read on light terminal backgrounds, making the output difficult to parse and potentially causing accessibility issues.

## Solution

The color adaptation system provides:
- **Automatic detection** of terminal background type
- **Manual override** options for specific use cases
- **Optimized color schemes** for both dark and light backgrounds
- **Consistent readability** across different terminal environments

## Usage

### Automatic Detection (Default)

By default, the CLI automatically detects your terminal background:

```bash
sync-ctl analyze my-project
```

The system uses several detection methods:
1. `COLORFGBG` environment variable analysis
2. Terminal program identification
3. Other terminal-specific heuristics

### Manual Override

You can manually specify the color scheme using the `--color-scheme` option:

#### Dark Terminal Theme
```bash
sync-ctl analyze my-project --color-scheme dark
```

#### Light Terminal Theme
```bash
sync-ctl analyze my-project --color-scheme light
```

#### Auto-Detection (Explicit)
```bash
sync-ctl analyze my-project --color-scheme auto
```

## Color Schemes

### Dark Background Theme
Optimized for terminals with dark backgrounds (most common):
- **Headers**: Bright white, bold
- **Borders**: Bright blue
- **Primary content**: Yellow for main items, green for secondary
- **Technology stack**: Blue for languages, magenta for frameworks, cyan for databases
- **Status indicators**: Green for success, yellow for warnings, red for errors

### Light Background Theme
Optimized for terminals with light backgrounds:
- **Headers**: Black, bold
- **Borders**: Blue
- **Primary content**: Bold red for main items, bold green for secondary
- **Technology stack**: Bold blue for languages, bold magenta for frameworks, bold cyan for databases
- **Status indicators**: Bold green for success, red for warnings, bold red for errors

## Supported Commands

The color adaptation feature works with all commands that produce colored output:

```bash
# Analysis commands
sync-ctl analyze --color-scheme light
sync-ctl dependencies --color-scheme dark
sync-ctl vulnerabilities --color-scheme auto

# Security commands
sync-ctl security --color-scheme light

# Any other command with colored output
```

## Environment Detection

The system attempts to detect your terminal background using:

### COLORFGBG Variable
The `COLORFGBG` environment variable (format: "foreground;background"):
- Background codes 0-6: Treated as dark
- Background codes 7-15: Treated as light

### Terminal Program Detection
Recognition of specific terminal applications:
- Terminal.app (macOS)
- iTerm2
- Windows Terminal
- And others

### Fallback Behavior
If detection fails, the system defaults to dark theme (most common).

## Testing Your Setup

Use the included test program to see how colors appear in your terminal:

```bash
cargo run --example test_color_adaptation
```

This will show you:
- Colors for dark theme
- Colors for light theme  
- Auto-detected theme for your terminal
- Side-by-side comparison

## Accessibility Benefits

### Improved Readability
- Proper contrast ratios for both backgrounds
- Consistent color intensity across themes
- Reduced eye strain in different lighting conditions

### Better Accessibility
- Colors work better with screen readers
- Higher contrast for users with visual impairments
- Consistent behavior across different terminal setups

## Technical Implementation

### Architecture
- `ColorAdapter`: Main color adaptation logic
- `ColorScheme`: Enum for Dark/Light themes
- Global state management for consistent colors
- Automatic initialization on first use

### Detection Methods
1. Environment variable parsing
2. Terminal program identification
3. Heuristic-based detection
4. Safe fallback to dark theme

### Integration
The color adapter is integrated into:
- Matrix view displays
- Detailed analysis output
- Security reports
- Error messages
- Progress indicators

## Examples

### Before (Fixed Colors)
```
Languages: JavaScript, TypeScript  # Always yellow, hard to read on light backgrounds
Status: ✅ Success                 # Always green, may be dim on light backgrounds
```

### After (Adaptive Colors)
**Dark Terminal:**
```
Languages: JavaScript, TypeScript  # Yellow text, good contrast
Status: ✅ Success                 # Green text, vibrant
```

**Light Terminal:**
```
Languages: JavaScript, TypeScript  # Bold blue text, excellent contrast
Status: ✅ Success                 # Bold green text, clearly visible
```

## Troubleshooting

### Colors Still Hard to Read?
1. Try manual override: `--color-scheme light` or `--color-scheme dark`
2. Check your terminal's contrast settings
3. Verify your terminal supports the color codes being used

### Auto-Detection Not Working?
1. Check if `COLORFGBG` environment variable is set: `echo $COLORFGBG`
2. Try manual override as a workaround
3. File an issue with your terminal details

### Terminal-Specific Issues
- **macOS Terminal**: Auto-detection should work with most themes
- **iTerm2**: Generally works well with auto-detection
- **Windows Terminal**: May require manual override in some cases
- **SSH/Remote**: Auto-detection may not work; use manual override

## Future Enhancements

Planned improvements include:
- More sophisticated terminal detection
- Additional color schemes (high contrast, colorblind-friendly)
- Per-command color preferences
- Configuration file support for color preferences

## Contributing

To improve color adaptation:
1. Test with your terminal setup
2. Report detection issues with terminal details
3. Suggest improvements for specific terminal types
4. Contribute detection logic for new terminals

## Related

- [Terminal Setup Guide](TERMINAL_SETUP.md)
- [Accessibility Features](ACCESSIBILITY.md)
- [Configuration Options](CONFIG.md)