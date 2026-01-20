# Linthis Brand Assets

## Logo & Icon Design

This directory contains the brand assets for linthis documentation.

### Files

- **logo.svg** - Main logo for documentation pages (200x200px)
- **favicon.svg** - Browser tab icon (64x64px)

### Design Concept

The linthis logo represents the core functionality of the tool:

#### Visual Elements

1. **Check Mark (✓)**
   - Represents code **linting** and validation
   - Central element showing correctness and quality
   - Bold stroke for visibility

2. **Code Lines**
   - Top left: Unformatted code lines (varying lengths)
   - Bottom right: Formatted code lines (aligned)
   - Represents the **formatter** functionality

3. **Multi-layer Design**
   - Multiple elements symbolize **multi-language support**
   - Different opacities create depth

#### Color Scheme

- **Primary Color**: Blue (#2196f3)
  - Represents clarity, trust, and professionalism
  - Associated with code quality and development tools
  - High contrast for visibility

- **Accent Color**: Light Blue (#64b5f6)
  - Used for secondary elements
  - Creates visual hierarchy

#### Typography

- Font: Arial, sans-serif (bold)
- Text: "LTS" at bottom (Lint This)
- Color: Blue with 80% opacity
- Size: Proportional to logo size

### Design Inspiration

Inspired by the ccgo logo style:
- Clean, modern aesthetic
- Circular/rounded design for logo
- Square with rounded corners for favicon
- Professional developer tool appearance

### Usage

#### In MkDocs

The logo and favicon are configured in `mkdocs.yml`:

```yaml
theme:
  name: material
  logo: assets/logo.svg
  favicon: assets/favicon.svg
  palette:
    - scheme: default
      primary: blue
      accent: light blue
```

#### Standalone Usage

Both SVG files can be used independently:
- As favicons for web applications
- In documentation
- In README files
- On project websites

### Color Palette

```
Primary Blue:     #2196f3
Light Blue:       #64b5f6
Background:       #2196f3 (10% opacity)
Border:           #2196f3
Text:             #2196f3 (80% opacity)
```

### Accessibility

- High contrast between logo and background
- Clear, recognizable shapes
- Scalable vector format (SVG)
- Works in both light and dark modes

### License

Same as linthis project - MIT License
