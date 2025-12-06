# Color Palette

Highlight: **#F7931A** — use sparingly for emphasis

---

## Background

| Name | Hex |
|------|-----|
| Primary | `#0D0D0D` |
| Secondary | `#141414` |
| Tertiary | `#1F1F1F` |
| Elevated | `#2A2A2A` |

---

## Accent

| Name | Hex |
|------|-----|
| Primary | `#8B7355` |
| Hover | `#A68B6A` |
| Muted | `#5C4D3D` |
| Subtle | `rgba(139, 115, 85, 0.15)` |

---

## Highlight

| Name | Hex |
|------|-----|
| Primary | `#F7931A` |
| Glow | `rgba(247, 147, 26, 0.25)` |

---

## Text

| Name | Hex |
|------|-----|
| Primary | `#E8E8E8` |
| Secondary | `#A3A3A3` |
| Muted | `#666666` |
| Inverse | `#0D0D0D` |

---

## Semantic

| Name | Hex |
|------|-----|
| Success | `#6B9B6B` |
| Error | `#B56B6B` |
| Warning | `#B5986B` |
| Info | `#6B8BB5` |

---

## CSS Variables

```css
:root {
  /* Background */
  --bg-primary: #0D0D0D;
  --bg-secondary: #141414;
  --bg-tertiary: #1F1F1F;
  --bg-elevated: #2A2A2A;

  /* Accent */
  --accent-primary: #8B7355;
  --accent-hover: #A68B6A;
  --accent-muted: #5C4D3D;
  --accent-subtle: rgba(139, 115, 85, 0.15);

  /* Highlight — use sparingly */
  --highlight: #F7931A;
  --highlight-glow: rgba(247, 147, 26, 0.25);

  /* Text */
  --text-primary: #E8E8E8;
  --text-secondary: #A3A3A3;
  --text-muted: #666666;
  --text-inverse: #0D0D0D;

  /* Semantic */
  --color-success: #6B9B6B;
  --color-error: #B56B6B;
  --color-warning: #B5986B;
  --color-info: #6B8BB5;
}
```

---

## Tailwind Config

```js
module.exports = {
  theme: {
    extend: {
      colors: {
        background: {
          primary: '#0D0D0D',
          secondary: '#141414',
          tertiary: '#1F1F1F',
          elevated: '#2A2A2A',
        },
        accent: {
          primary: '#8B7355',
          hover: '#A68B6A',
          muted: '#5C4D3D',
          subtle: 'rgba(139, 115, 85, 0.15)',
        },
        highlight: {
          DEFAULT: '#F7931A',
          glow: 'rgba(247, 147, 26, 0.25)',
        },
        text: {
          primary: '#E8E8E8',
          secondary: '#A3A3A3',
          muted: '#666666',
          inverse: '#0D0D0D',
        },
      },
    },
  },
};
```

---

## Usage Guidelines

- **Highlight (#F7931A)**: Active states, important CTAs, badges, notifications, selected items
- **Accent (#8B7355)**: Borders, secondary buttons, icons, dividers, hover states
- **Background tiers**: Layer UI depth — primary → secondary → tertiary → elevated
- **Text tiers**: Primary for headings/body, secondary for descriptions, muted for captions