use ratatui::style::Color;

/// Anthropic orange-based pixel theme
pub struct Theme {
    #[allow(dead_code)]
    pub name: &'static str,
    pub primary: Color,
    pub bright: Color,
    pub dim: Color,
    pub bg: Color,
    pub surface: Color,
    pub text: Color,
    pub text_dim: Color,
    #[allow(dead_code)]
    pub accent: Color,
    pub spectrum_colors: Vec<Color>,
}

impl Theme {
    /// Default Anthropic orange theme
    pub fn anthropic() -> Self {
        Self {
            name: "anthropic",
            primary: Color::Rgb(227, 137, 62), // #E3893E - Anthropic orange
            bright: Color::Rgb(255, 170, 80),  // Bright orange
            dim: Color::Rgb(140, 85, 40),      // Dim orange
            bg: Color::Rgb(20, 15, 10),        // Near-black warm
            surface: Color::Rgb(35, 25, 18),   // Dark surface
            text: Color::Rgb(230, 210, 190),   // Warm white
            text_dim: Color::Rgb(120, 100, 80), // Muted text
            accent: Color::Rgb(255, 200, 100), // Gold accent
            spectrum_colors: vec![
                Color::Rgb(140, 85, 40),   // Low - dim orange
                Color::Rgb(180, 110, 50),  //
                Color::Rgb(210, 130, 55),  //
                Color::Rgb(227, 137, 62),  // Mid - primary orange
                Color::Rgb(240, 155, 70),  //
                Color::Rgb(255, 170, 80),  // High - bright orange
                Color::Rgb(255, 190, 100), //
                Color::Rgb(255, 210, 130), // Peak - golden
            ],
        }
    }

    /// Get spectrum color for a given intensity (0.0 - 1.0)
    pub fn spectrum_color(&self, intensity: f32) -> Color {
        let idx = (intensity * (self.spectrum_colors.len() - 1) as f32).round() as usize;
        self.spectrum_colors[idx.min(self.spectrum_colors.len() - 1)]
    }
}
