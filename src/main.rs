use std::error::Error;
use std::fs::File;
use std::io::Read;
use image::{RgbaImage, Rgba};
use serde::Deserialize;
use rusttype::{Font as RustFont, Scale};
use layer_trait::SourceLayer;
use csscolorparser::parse as parse_color;
use font_kit::source::SystemSource;
use font_kit::properties::{Properties, Weight, Style};
use font_kit::family_name::FamilyName;

mod ai_handler;
mod layer_trait;
use ai_handler::AiData;

#[derive(Deserialize)]
struct Size {
    width: u32,
    height: u32,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
enum HorizontalAlign {
    Left,
    Center,
    Right,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
enum VerticalAlign {
    Top,
    Middle,
    Bottom,
    Below,
    Above,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
enum RelativeTo {
    Canvas,
    Layer(String),
}

#[derive(Deserialize, Clone)]
struct Position {
    x: u32,
    y: u32,
    #[serde(default = "default_relative_to")]
    relative_to: RelativeTo,
}

fn default_relative_to() -> RelativeTo {
    RelativeTo::Canvas
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
enum FontWeight {
    Normal,
    Bold,
    #[serde(rename = "100")]
    Weight100,
    #[serde(rename = "200")]
    Weight200,
    #[serde(rename = "300")]
    Weight300,
    #[serde(rename = "400")]
    Weight400,
    #[serde(rename = "500")]
    Weight500,
    #[serde(rename = "600")]
    Weight600,
    #[serde(rename = "700")]
    Weight700,
    #[serde(rename = "800")]
    Weight800,
    #[serde(rename = "900")]
    Weight900,
}

impl FontWeight {
    fn to_font_kit_weight(&self) -> Weight {
        match self {
            FontWeight::Normal => Weight::NORMAL,
            FontWeight::Bold => Weight::BOLD,
            FontWeight::Weight100 => Weight::THIN,
            FontWeight::Weight200 => Weight::EXTRA_LIGHT,
            FontWeight::Weight300 => Weight::LIGHT,
            FontWeight::Weight400 => Weight::NORMAL,
            FontWeight::Weight500 => Weight::MEDIUM,
            FontWeight::Weight600 => Weight::SEMIBOLD,
            FontWeight::Weight700 => Weight::BOLD,
            FontWeight::Weight800 => Weight::EXTRA_BOLD,
            FontWeight::Weight900 => Weight::BLACK,
        }
    }
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

impl FontStyle {
    fn to_font_kit_style(&self) -> Style {
        match self {
            FontStyle::Normal => Style::Normal,
            FontStyle::Italic => Style::Italic,
            FontStyle::Oblique => Style::Oblique,
        }
    }
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
enum FontDecoration {
    None,
    Underline,
    LineThrough,
    Overline,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
enum TextAlignment {
    Left,
    Center,
    Right,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
enum TextJustification {
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Deserialize, Clone)]
struct FontSpec {
    family: String,
    size: f32,
    color: String,
    #[serde(default = "default_font_weight")]
    weight: FontWeight,
    #[serde(default = "default_font_style")]
    style: FontStyle,
    #[serde(default = "default_font_decoration")]
    decoration: FontDecoration,
}

fn default_font_weight() -> FontWeight {
    FontWeight::Normal
}

fn default_font_style() -> FontStyle {
    FontStyle::Normal
}

fn default_font_decoration() -> FontDecoration {
    FontDecoration::None
}

impl FontSpec {
    fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Validate color format
        parse_color(&self.color)?;
        
        // Validate font size
        if self.size <= 0.0 {
            return Err("Font size must be positive".into());
        }
        
        // Try to load the font to validate it exists
        let source = SystemSource::new();
        let properties = Properties {
            weight: self.weight.to_font_kit_weight(),
            style: self.style.to_font_kit_style(),
            ..Properties::default()
        };

        source.select_best_match(&[FamilyName::Title(self.family.clone())], &properties)
            .map_err(|_| format!("Font family '{}' not found in system fonts", self.family))?;
        
        Ok(())
    }

    fn load_font(&self) -> Result<RustFont<'static>, Box<dyn std::error::Error>> {
        let source = SystemSource::new();
        let properties = Properties {
            weight: self.weight.to_font_kit_weight(),
            style: self.style.to_font_kit_style(),
            ..Properties::default()
        };

        let handle = source.select_best_match(&[FamilyName::Title(self.family.clone())], &properties)
            .map_err(|_| format!("Font '{}' not found", self.family))?;

        let font = handle.load()
            .map_err(|_| "Failed to load font")?;

        let font_data = font.copy_font_data()
            .ok_or("Failed to get font data")?;

        RustFont::try_from_vec(font_data.to_vec())
            .ok_or_else(|| "Failed to create font".into())
    }

    fn draw_decoration(&self, canvas: &mut RgbaImage, text_color: Rgba<u8>, x: u32, y: u32, width: u32, height: u32) {
        let line_thickness = (self.size / 16.0).max(1.0) as u32;
        
        match self.decoration {
            FontDecoration::None => {},
            FontDecoration::Underline => {
                // Draw underline at the bottom of text
                let line_y = y + height + line_thickness;
                draw_horizontal_line(canvas, text_color, x, line_y, width, line_thickness);
            },
            FontDecoration::LineThrough => {
                // Draw line through middle of text
                let line_y = y + (height / 2);
                draw_horizontal_line(canvas, text_color, x, line_y, width, line_thickness);
            },
            FontDecoration::Overline => {
                // Draw line above text
                let line_y = y.saturating_sub(line_thickness * 2);
                draw_horizontal_line(canvas, text_color, x, line_y, width, line_thickness);
            },
        }
    }
}

fn draw_horizontal_line(canvas: &mut RgbaImage, color: Rgba<u8>, x: u32, y: u32, width: u32, thickness: u32) {
    for dy in 0..thickness {
        let line_y = y + dy;
        if line_y >= canvas.height() {
            break;
        }
        for dx in 0..width {
            let line_x = x + dx;
            if line_x >= canvas.width() {
                break;
            }
            canvas.put_pixel(line_x, line_y, color);
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum LayoutType {
    Vertical,
    Horizontal,
    Grid,
}

#[derive(Deserialize)]
struct GroupPosition {
    x: u32,
    y: u32,
}

#[derive(Deserialize)]
struct DistributionConfig {
    #[serde(default)]
    bounds: Option<DistributionBounds>,
}

#[derive(Deserialize)]
struct DistributionBounds {
    width: u32,
    height: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum GroupAlignment {
    Left,
    Center,
    Right,
    Top,
    Bottom,
}

fn default_group_alignment() -> GroupAlignment {
    GroupAlignment::Left
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum GroupJustification {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

fn default_group_justification() -> GroupJustification {
    GroupJustification::Start
}

fn default_spacing() -> u32 {
    0
}

#[derive(Deserialize)]
struct GroupLayout {
    #[serde(rename = "type")]
    layout_type: LayoutType,
    position: GroupPosition,
    #[serde(default = "default_spacing")]
    spacing: u32,
    #[serde(default = "default_columns")]
    columns: u32,
    #[serde(default)]
    distribution: Option<DistributionConfig>,
    #[serde(default = "default_group_alignment")]
    alignment: GroupAlignment,
    #[serde(default = "default_group_justification")]
    justification: GroupJustification,
}

fn default_columns() -> u32 {
    1
}

#[derive(Deserialize, Clone)]
struct LayerInfo {
    name: String,
    #[serde(flatten)]
    position: Option<Position>,
}

#[derive(Deserialize)]
struct Group {
    #[allow(dead_code)]
    name: String,
    layout: GroupLayout,
    layers: Vec<Layer>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum SourceType {
    AI,
}

#[derive(Deserialize)]
struct SourceFile {
    path: String,
    #[serde(rename = "type")]
    file_type: SourceType,
}

#[derive(Deserialize)]
struct Template {
    size: Size,
    background: String,
    source: Option<String>,
    groups: Vec<Group>,
}

// Helper struct to store layer dimensions
struct LayerDimensions {
    width: u32,
    height: u32,
}

trait GetDimensions {
    fn get_dimensions(&self) -> Result<LayerDimensions, Box<dyn std::error::Error>>;
}

impl GetDimensions for Layer {
    fn get_dimensions(&self) -> Result<LayerDimensions, Box<dyn std::error::Error>> {
        match self {
            Layer::Text(text_layer) => {
                let font = text_layer.font.load_font()?;
                let scale = Scale::uniform(text_layer.font.size);
                
                let glyphs: Vec<_> = font
                    .layout(&text_layer.text, scale, rusttype::point(0.0, 0.0))
                    .collect();
                
                let width = glyphs
                    .iter()
                    .filter_map(|g| g.pixel_bounding_box())
                    .fold(0, |acc, bbox| acc + bbox.width()) as u32;

                let height = glyphs
                    .iter()
                    .filter_map(|g| g.pixel_bounding_box())
                    .fold(0, |acc, bbox| acc.max(bbox.height())) as u32;

                Ok(LayerDimensions { width, height })
            },
            Layer::Image(image_layer) => {
                let img = image::open(&image_layer.source)?;
                let width = (img.width() as f32 * image_layer.scale) as u32;
                let height = (img.height() as f32 * image_layer.scale) as u32;
                Ok(LayerDimensions { width, height })
            },
        }
    }
}

impl Group {
    fn calculate_positions(&self, layers_info: &[(LayerDimensions, &LayerInfo)]) -> Vec<Position> {
        let mut positions = Vec::new();
        let current_x = self.layout.position.x;
        let current_y = self.layout.position.y;

        // Calculate total dimensions
        let (total_width, total_height) = match self.layout.layout_type {
            LayoutType::Horizontal => {
                let width = layers_info.iter()
                    .map(|(dims, _)| dims.width)
                    .sum::<u32>() + (layers_info.len().saturating_sub(1) as u32 * self.layout.spacing);
                let height = layers_info.iter()
                    .map(|(dims, _)| dims.height)
                    .max()
                    .unwrap_or(0);
                (width, height)
            },
            LayoutType::Grid => {
                let columns = self.layout.columns as usize;
                let rows = (layers_info.len() + columns - 1) / columns;
                
                let max_width_per_column: Vec<u32> = (0..columns)
                    .map(|col| {
                        layers_info.iter()
                            .skip(col)
                            .step_by(columns)
                            .map(|(dims, _)| dims.width)
                            .max()
                            .unwrap_or(0)
                    })
                    .collect();
                
                let max_height_per_row: Vec<u32> = (0..rows)
                    .map(|row| {
                        layers_info.iter()
                            .skip(row * columns)
                            .take(columns)
                            .map(|(dims, _)| dims.height)
                            .max()
                            .unwrap_or(0)
                    })
                    .collect();

                let width = max_width_per_column.iter().sum::<u32>() + 
                    (columns.saturating_sub(1) as u32 * self.layout.spacing);
                let height = max_height_per_row.iter().sum::<u32>() + 
                    (rows.saturating_sub(1) as u32 * self.layout.spacing);
                (width, height)
            },
            LayoutType::Vertical => {
                let width = layers_info.iter()
                    .map(|(dims, _)| dims.width)
                    .max()
                    .unwrap_or(0);
                let height = layers_info.iter()
                    .map(|(dims, _)| dims.height)
                    .sum::<u32>() + (layers_info.len().saturating_sub(1) as u32 * self.layout.spacing);
                (width, height)
            },
        };

        // Get container bounds from distribution config or use total dimensions
        let container_bounds = if let Some(dist_config) = &self.layout.distribution {
            if let Some(bounds) = &dist_config.bounds {
                (bounds.width, bounds.height)
            } else {
                (total_width, total_height)
            }
        } else {
            (total_width, total_height)
        };

        // Apply global alignment
        let (base_x, base_y) = match self.layout.alignment {
            GroupAlignment::Left => (current_x, current_y),
            GroupAlignment::Center => (
                current_x + (container_bounds.0.saturating_sub(total_width)) / 2,
                current_y + (container_bounds.1.saturating_sub(total_height)) / 2
            ),
            GroupAlignment::Right => (
                current_x + container_bounds.0.saturating_sub(total_width),
                current_y
            ),
            GroupAlignment::Top => (
                current_x,
                current_y
            ),
            GroupAlignment::Bottom => (
                current_x,
                current_y + container_bounds.1.saturating_sub(total_height)
            ),
        };

        // Calculate spacing based on justification
        let (init_spacing, item_spacing) = match self.layout.justification {
            GroupJustification::Start => (0, self.layout.spacing),
            GroupJustification::Center => {
                let total_space = match self.layout.layout_type {
                    LayoutType::Horizontal => container_bounds.0.saturating_sub(total_width),
                    LayoutType::Vertical => container_bounds.1.saturating_sub(total_height),
                    LayoutType::Grid => 0, // Grid handles spacing differently
                };
                (total_space / 2, self.layout.spacing)
            },
            GroupJustification::End => {
                let total_space = match self.layout.layout_type {
                    LayoutType::Horizontal => container_bounds.0.saturating_sub(total_width),
                    LayoutType::Vertical => container_bounds.1.saturating_sub(total_height),
                    LayoutType::Grid => 0,
                };
                (total_space, self.layout.spacing)
            },
            GroupJustification::SpaceBetween => {
                let count = layers_info.len().saturating_sub(1).max(1);
                let total_space = match self.layout.layout_type {
                    LayoutType::Horizontal => container_bounds.0.saturating_sub(total_width),
                    LayoutType::Vertical => container_bounds.1.saturating_sub(total_height),
                    LayoutType::Grid => 0,
                };
                (0, total_space / count as u32)
            },
            GroupJustification::SpaceAround => {
                let count = layers_info.len() + 1;
                let total_space = match self.layout.layout_type {
                    LayoutType::Horizontal => container_bounds.0.saturating_sub(total_width),
                    LayoutType::Vertical => container_bounds.1.saturating_sub(total_height),
                    LayoutType::Grid => 0,
                };
                let spacing = total_space / count as u32;
                (spacing, spacing)
            },
            GroupJustification::SpaceEvenly => {
                let count = layers_info.len() + 2;
                let total_space = match self.layout.layout_type {
                    LayoutType::Horizontal => container_bounds.0.saturating_sub(total_width),
                    LayoutType::Vertical => container_bounds.1.saturating_sub(total_height),
                    LayoutType::Grid => 0,
                };
                let spacing = total_space / count as u32;
                (spacing, spacing)
            },
        };

        // Position elements based on layout type and justification
        match self.layout.layout_type {
            LayoutType::Grid => {
                let columns = self.layout.columns as usize;
                let mut x = base_x;
                let mut y = base_y;
                let mut col = 0;

                for (dims, _) in layers_info {
                    positions.push(Position {
                        x,
                        y,
                        relative_to: RelativeTo::Canvas,
                    });

                    col += 1;
                    if col >= columns {
                        // Move to next row
                        col = 0;
                        x = base_x;
                        y += dims.height + item_spacing;
                    } else {
                        x += dims.width + item_spacing;
                    }
                }
            },
            LayoutType::Vertical => {
                let mut y = base_y + init_spacing;
                for (dims, _) in layers_info {
                    let x = match self.layout.alignment {
                        GroupAlignment::Left => base_x,
                        GroupAlignment::Center => base_x + (container_bounds.0.saturating_sub(dims.width)) / 2,
                        GroupAlignment::Right => base_x + container_bounds.0.saturating_sub(dims.width),
                        _ => base_x,
                    };

                    positions.push(Position {
                        x,
                        y,
                        relative_to: RelativeTo::Canvas,
                    });
                    y += dims.height + item_spacing;
                }
            },
            LayoutType::Horizontal => {
                let mut x = base_x + init_spacing;
                for (dims, _) in layers_info {
                    let y = match self.layout.alignment {
                        GroupAlignment::Top => base_y,
                        GroupAlignment::Center => base_y + (container_bounds.1.saturating_sub(dims.height)) / 2,
                        GroupAlignment::Bottom => base_y + container_bounds.1.saturating_sub(dims.height),
                        _ => base_y,
                    };

                    positions.push(Position {
                        x,
                        y,
                        relative_to: RelativeTo::Canvas,
                    });
                    x += dims.width + item_spacing;
                }
            },
        }

        // Handle relative positioning
        let mut relative_adjustments = Vec::new();
        for (i, pos) in positions.iter().enumerate() {
            if let RelativeTo::Layer(ref layer_name) = pos.relative_to {
                if let Some((ref_idx, _)) = layers_info.iter()
                    .enumerate()
                    .find(|(_, (_, info))| info.name == *layer_name)
                {
                    relative_adjustments.push((i, ref_idx));
                }
            }
        }

        for (target_idx, ref_idx) in relative_adjustments {
            let ref_pos = positions[ref_idx].clone();
            positions[target_idx].x = ref_pos.x;
            positions[target_idx].y = ref_pos.y;
        }

        positions
    }
}

#[derive(Deserialize, Clone)]
struct TextLayer {
    #[serde(rename = "type")]
    layer_type: String,
    #[serde(flatten)]
    info: LayerInfo,
    text: String,
    font: FontSpec,
    alignment: TextAlignment,
    #[serde(default = "default_text_justification")]
    justification: TextJustification,
}

fn default_text_justification() -> TextJustification {
    TextJustification::Left
}

impl TextLayer {
    fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.layer_type != "text" {
            return Err("Invalid layer type for text layer".into());
        }
        
        if self.text.is_empty() {
            return Err("Text content cannot be empty".into());
        }
        
        self.font.validate()?;
        
        Ok(())
    }

    fn draw(&self, canvas: &mut RgbaImage, position: &Position) -> Result<(), Box<dyn std::error::Error>> {
        let font = self.font.load_font()?;
        let text_color = parse_color(&self.font.color)?;
        let rgba_color = Rgba([
            (text_color.r * 255.0) as u8,
            (text_color.g * 255.0) as u8,
            (text_color.b * 255.0) as u8,
            (text_color.a * 255.0) as u8,
        ]);
        let scale = Scale::uniform(self.font.size);
        let v_metrics = font.v_metrics(scale);

        // Calculate text dimensions
        let text = self.text.replace("{{name}}", "World");
        let glyphs: Vec<_> = font
            .layout(&text, scale, rusttype::point(0.0, 0.0))
            .collect();
        
        let text_width = glyphs
            .iter()
            .filter_map(|g| g.pixel_bounding_box())
            .fold(0, |acc, bbox| acc + bbox.width()) as u32;

        let text_height = glyphs
            .iter()
            .filter_map(|g| g.pixel_bounding_box())
            .fold(0, |acc, bbox| acc.max(bbox.height())) as u32;

        let x_position = match self.alignment {
            TextAlignment::Center => position.x.saturating_sub(text_width / 2),
            TextAlignment::Right => position.x.saturating_sub(text_width),
            TextAlignment::Left => position.x,
        };

        // Apply justification spacing
        let justified_spacing = match self.justification {
            TextJustification::Justify => {
                let words = text.split_whitespace().count();
                if words > 1 {
                    Some((canvas.width() - text_width) as f32 / (words - 1) as f32)
                } else {
                    None
                }
            },
            _ => None,
        };

        // Layout the text with justification if needed
        let mut current_x = x_position as f32;
        let y_position = position.y;
        let words: Vec<_> = text.split_whitespace().collect();
        
        for (i, word) in words.iter().enumerate() {
            let glyphs: Vec<_> = font
                .layout(
                    word,
                    scale,
                    rusttype::point(current_x, y_position as f32 + v_metrics.ascent),
                )
                .collect();

            // Get word dimensions for decoration
            let word_width = glyphs
                .iter()
                .filter_map(|g| g.pixel_bounding_box())
                .fold(0, |acc, bbox| acc + bbox.width()) as u32;

            // Draw the word
            for glyph in glyphs {
                if let Some(bounding_box) = glyph.pixel_bounding_box() {
                    glyph.draw(|x, y, v| {
                        let x = (x as i32 + bounding_box.min.x) as u32;
                        let y = (y as i32 + bounding_box.min.y) as u32;
                        if x < canvas.width() && y < canvas.height() {
                            canvas.put_pixel(
                                x,
                                y,
                                Rgba([
                                    rgba_color[0],
                                    rgba_color[1],
                                    rgba_color[2],
                                    ((rgba_color[3] as f32) * v) as u8,
                                ]),
                            );
                        }
                    });
                }
            }

            // Draw decoration for this word
            self.font.draw_decoration(
                canvas,
                rgba_color,
                current_x as u32,
                position.y,
                word_width,
                text_height,
            );

            // Update x position for next word
            if i < words.len() - 1 {
                current_x += word_width as f32 + if let Some(spacing) = justified_spacing {
                    spacing
                } else {
                    scale.x // default space width
                };
            }
        }

        Ok(())
    }
}

#[derive(Deserialize)]
struct ImageLayer {
    #[serde(rename = "type")]
    layer_type: String,
    #[serde(flatten)]
    info: LayerInfo,
    source: String,
    scale: f32,
}

impl ImageLayer {
    fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.layer_type != "image" {
            return Err("Invalid layer type for image layer".into());
        }
        
        if self.scale <= 0.0 {
            return Err("Scale must be positive".into());
        }
        
        if !std::path::Path::new(&self.source).exists() {
            return Err(format!("Image file not found: {}", self.source).into());
        }
        
        Ok(())
    }

    fn draw(&self, canvas: &mut RgbaImage, position: &Position) -> Result<(), Box<dyn std::error::Error>> {
        let mut overlay = image::open(&self.source)?;
        
        // Apply scaling if needed
        if self.scale != 1.0 {
            let new_width = (overlay.width() as f32 * self.scale) as u32;
            let new_height = (overlay.height() as f32 * self.scale) as u32;
            overlay = overlay.resize(new_width, new_height, image::imageops::FilterType::Lanczos3);
        }

        image::imageops::overlay(
            canvas,
            &overlay,
            position.x as i64,
            position.y as i64,
        );

        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Layer {
    Text(TextLayer),
    Image(ImageLayer),
}

impl Template {
    fn process(&self) -> Result<RgbaImage, Box<dyn std::error::Error>> {
        println!("Processing template");
        // Create a new image with the specified size and background color
        let mut canvas = RgbaImage::new(self.size.width, self.size.height);
        let bg_color = parse_color(&self.background)?;
        let bg_rgba = Rgba([
            (bg_color.r * 255.0) as u8,
            (bg_color.g * 255.0) as u8,
            (bg_color.b * 255.0) as u8,
            (bg_color.a * 255.0) as u8,
        ]);

        // Fill background
        for pixel in canvas.pixels_mut() {
            *pixel = bg_rgba;
        }

        // Load source file if specified
        let source_data = if let Some(source) = &self.source {
            println!("Loading source file: {}", source);
            if source.ends_with(".ai") {
                match AiData::new(source, Some(source)) {
                    Ok(data) => {
                        println!("Successfully loaded source file");
                        Some(SourceData::Ai(data))
                    }
                    Err(e) => {
                        println!("Error loading source file: {:?}", e);
                        return Err(format!("Failed to load source file: {}", e).into());
                    }
                }
            } else {
                return Err(format!("Unsupported source file type: {}", source).into());
            }
        } else {
            None
        };

        // If we have a source file, validate that all required layers exist
        if let Some(ref source) = source_data {
            // Collect all text layer names that need to be found in the source
            let required_layer_names: Vec<String> = self.groups.iter()
                .flat_map(|group| &group.layers)
                .filter_map(|layer| {
                    if let Layer::Text(text) = layer {
                        Some(text.info.name.clone())
                    } else {
                        None
                    }
                })
                .collect();

            // Check each required layer exists in the source
            for layer_name in &required_layer_names {
                let layer_exists = match source {
                    SourceData::Ai(ai) => ai.get_layer_by_name(layer_name).is_some(),
                };
                if !layer_exists {
                    return Err(format!("Required layer '{}' not found in source file", layer_name).into());
                }
            }
        }

        // Process each group
        for group in &self.groups {
            let mut layer_dimensions = Vec::new();
            
            // Calculate dimensions for each layer
            for layer in &group.layers {
                let dimensions = layer.get_dimensions()?;
                let layer_info = match layer {
                    Layer::Text(text) => &text.info,
                    Layer::Image(image) => &image.info,
                };
                layer_dimensions.push((dimensions, layer_info));
            }

            // Calculate positions for all layers in the group
            let positions = group.calculate_positions(&layer_dimensions);

            // Draw each layer
            for (layer, position) in group.layers.iter().zip(positions.iter()) {
                match layer {
                    Layer::Text(text) => {
                        let mut modified_text = text.clone();
                        
                        // Try to get text content from source file
                        if let Some(ref source) = source_data {
                            let source_layer = match source {
                                SourceData::Ai(ai) => ai.get_layer_by_name(&text.info.name),
                            };
                            if source_layer.is_none() {
                                return Err(format!("Required layer '{}' not found in source file", text.info.name).into());
                            }
                        }

                        modified_text.draw(&mut canvas, position)?;
                    }
                    Layer::Image(image) => {
                        image.draw(&mut canvas, position)?;
                    }
                }
            }
        }

        Ok(canvas)
    }
}

enum SourceData {
    Ai(AiData),
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum TemplateLayer {
    Text(TextLayer),
    Image(ImageLayer),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create output directory if it doesn't exist
    std::fs::create_dir_all("output")?;

    // Load and parse the template
    let mut template_file = File::open("templates/ai.json")?;
    let mut template_contents = String::new();
    template_file.read_to_string(&mut template_contents)?;
    let template: Template = serde_json::from_str(&template_contents)?;

    // Process the template
    let result_image = template.process()?;

    // Save the result
    result_image.save("output/result.png")?;
    println!("Image has been created successfully in output/result.png!");
    
    Ok(())
}

