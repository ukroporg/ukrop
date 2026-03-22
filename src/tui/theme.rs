use ratatui::style::{Color, Modifier, Style};

use crate::config::{Config, ThemePreset};

pub struct Theme {
    // Borders
    pub border_active: Style,
    pub border_inactive: Style,
    // Search bar
    pub prompt: Style,
    pub cursor: Style,
    // List items
    pub selected: Style,
    pub age_recent: Style,
    pub age_mid: Style,
    pub age_old: Style,
    pub missing: Style,
    pub highlight_modifier: Modifier,
    pub highlight_fg: Color,
    pub favorite: Style,
    // Headers & status
    pub section_header: Style,
    pub status_hint: Style,
    pub flash: Style,
    // Dialog
    pub dialog_border: Style,
    pub dialog_key: Style,
    pub dialog_desc: Style,
    // Layout
    pub left_panel_pct: u16,
    pub cd_panel_pct: u16,
}

struct Palette {
    border: Color,
    highlight: Color,
    age_recent: Color,
    age_mid: Color,
    age_old: Color,
    header: Color,
}

fn palette_for(preset: &ThemePreset) -> Palette {
    match preset {
        ThemePreset::Default => Palette {
            border: Color::Yellow,
            highlight: Color::Cyan,
            age_recent: Color::Green,
            age_mid: Color::White,
            age_old: Color::DarkGray,
            header: Color::Cyan,
        },
        ThemePreset::Light => Palette {
            border: Color::Blue,
            highlight: Color::Magenta,
            age_recent: Color::DarkGray,
            age_mid: Color::Black,
            age_old: Color::DarkGray,
            header: Color::Blue,
        },
        ThemePreset::Nord => Palette {
            border: Color::Indexed(67),   // ~#5E81AC
            highlight: Color::Indexed(116), // ~#88C0D0
            age_recent: Color::Indexed(108), // ~#A3BE8C
            age_mid: Color::White,
            age_old: Color::DarkGray,
            header: Color::Indexed(67),
        },
        ThemePreset::Solarized => Palette {
            border: Color::Indexed(33),   // ~#268BD2
            highlight: Color::Indexed(37), // ~#2AA198
            age_recent: Color::Indexed(64), // ~#859900
            age_mid: Color::White,
            age_old: Color::DarkGray,
            header: Color::Indexed(33),
        },
        ThemePreset::Monochrome => Palette {
            border: Color::White,
            highlight: Color::White,
            age_recent: Color::White,
            age_mid: Color::Gray,
            age_old: Color::DarkGray,
            header: Color::White,
        },
        ThemePreset::Dracula => Palette {
            border: Color::Indexed(141),  // #BD93F9 purple
            highlight: Color::Indexed(84), // #50FA7B green
            age_recent: Color::Indexed(84), // green
            age_mid: Color::Indexed(231),  // #F8F8F2 foreground
            age_old: Color::Indexed(61),   // #6272A4 comment
            header: Color::Indexed(212),   // #FF79C6 pink
        },
        ThemePreset::Gruvbox => Palette {
            border: Color::Indexed(214),  // #FABD2F yellow
            highlight: Color::Indexed(108), // #8EC07C aqua
            age_recent: Color::Indexed(142), // #B8BB26 green
            age_mid: Color::Indexed(223),  // #EBDBB2 fg
            age_old: Color::Indexed(246),  // #A89984 gray
            header: Color::Indexed(214),   // yellow
        },
        ThemePreset::Catppuccin => Palette {
            border: Color::Indexed(183),  // #CBA6F7 mauve
            highlight: Color::Indexed(158), // #A6E3A1 green
            age_recent: Color::Indexed(158), // green
            age_mid: Color::Indexed(189),  // #CDD6F4 text
            age_old: Color::Indexed(103),  // #6C7086 overlay0
            header: Color::Indexed(183),   // mauve
        },
        ThemePreset::TokyoNight => Palette {
            border: Color::Indexed(75),   // #7AA2F7 blue
            highlight: Color::Indexed(180), // #E0AF68 yellow
            age_recent: Color::Indexed(114), // #9ECE6A green
            age_mid: Color::Indexed(189),  // #C0CAF5 fg
            age_old: Color::Indexed(60),   // #565F89 comment
            header: Color::Indexed(75),    // blue
        },
        ThemePreset::Kanagawa => Palette {
            border: Color::Indexed(110),  // #7E9CD8 crystal blue
            highlight: Color::Indexed(222), // #DCA561 autumn yellow
            age_recent: Color::Indexed(114), // #76946A autumn green
            age_mid: Color::Indexed(188),  // #DCD7BA fuji white
            age_old: Color::Indexed(102),  // #727169 fuji gray
            header: Color::Indexed(175),   // #D27E99 sakura pink
        },
        ThemePreset::Everforest => Palette {
            border: Color::Indexed(108),  // #A7C080 green
            highlight: Color::Indexed(214), // #DBBC7F yellow
            age_recent: Color::Indexed(108), // green
            age_mid: Color::Indexed(187),  // #D3C6AA fg
            age_old: Color::Indexed(245),  // #859289 gray
            header: Color::Indexed(174),   // #E67E80 red
        },
        ThemePreset::Rose => Palette {
            border: Color::Indexed(168),  // #EB6F92 love
            highlight: Color::Indexed(189), // #C4A7E7 iris
            age_recent: Color::Indexed(79), // #31748F pine
            age_mid: Color::Indexed(254),  // #E0DEF4 text
            age_old: Color::Indexed(103),  // #908CAA subtle
            header: Color::Indexed(168),   // love
        },
    }
}

impl Theme {
    pub fn from_config(cfg: &Config) -> Self {
        let p = palette_for(&cfg.theme.preset);

        let mut selected = Style::default().fg(p.border);
        if cfg.theme.selection_bold {
            selected = selected.add_modifier(Modifier::BOLD);
        }

        let mut highlight_modifier = Modifier::empty();
        if cfg.theme.match_underline {
            highlight_modifier = Modifier::UNDERLINED;
        }

        let mut favorite = Style::default().fg(Color::Magenta);
        if cfg.theme.favorite_italic {
            favorite = favorite.add_modifier(Modifier::ITALIC);
        }

        let left_pct = cfg.layout.left_panel_pct.clamp(5, 50);
        let cd_pct = cfg.layout.cd_panel_pct.clamp(10, 90);

        Theme {
            border_active: Style::default().fg(p.border),
            border_inactive: Style::default().fg(Color::DarkGray),
            prompt: Style::default().fg(p.border),
            cursor: Style::default().fg(Color::Black).bg(Color::White),
            selected,
            age_recent: Style::default().fg(p.age_recent),
            age_mid: Style::default().fg(p.age_mid),
            age_old: Style::default().fg(p.age_old),
            missing: Style::default().fg(Color::Red),
            highlight_modifier,
            highlight_fg: p.highlight,
            favorite,
            section_header: Style::default().fg(p.header).add_modifier(Modifier::BOLD),
            status_hint: Style::default().fg(Color::DarkGray),
            flash: Style::default().fg(p.border).add_modifier(Modifier::BOLD),
            dialog_border: Style::default().fg(p.header),
            dialog_key: Style::default().fg(p.border).add_modifier(Modifier::BOLD),
            dialog_desc: Style::default().fg(Color::White),
            left_panel_pct: left_pct,
            cd_panel_pct: cd_pct,
        }
    }

    pub fn age_style(&self, last_time: i64) -> Style {
        let now = chrono::Utc::now().timestamp();
        let age_secs = now - last_time;
        let one_day = 86400;
        if age_secs < 7 * one_day {
            self.age_recent
        } else if age_secs < 30 * one_day {
            self.age_mid
        } else {
            self.age_old
        }
    }

    pub fn item_style(&self, is_selected: bool, not_exists: bool, last_time: i64) -> Style {
        if not_exists {
            self.missing.add_modifier(Modifier::CROSSED_OUT)
        } else if is_selected {
            self.selected
        } else {
            self.age_style(last_time)
        }
    }

    pub fn highlight_style(&self, base: Style) -> Style {
        base.fg(self.highlight_fg).add_modifier(self.highlight_modifier)
    }
}
