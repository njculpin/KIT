use image::{Rgba, RgbaImage};
use rusttype::{Font as RustFont, Scale};
use serde::Deserialize;
use std::fs::File;
use std::io::Read;
use csscolorparser::parse as parse_color;
use font_kit::source::SystemSource;
use font_kit::properties::{Properties, Weight, Style};
use font_kit::family_name::FamilyName;

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

#[derive(Deserialize)]
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

#[derive(Deserialize)]
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

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum FontDecoration {
    None,
    Underline,
    LineThrough,
    Overline,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum TextAlignment {
    Left,
    Center,
    Right,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum TextJustification {
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Deserialize)]
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
    #[serde(default = "default_align")]
    align: Align,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Align {
    Left,
    Center,
    Right,
}

fn default_align() -> Align {
    Align::Left
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Distribution {
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Deserialize)]
struct DistributionBounds {
    width: u32,
    height: u32,
}

#[derive(Deserialize)]
struct DistributionConfig {
    #[serde(default)]
    horizontal: Option<Distribution>,
    #[serde(default)]
    vertical: Option<Distribution>,
    #[serde(default)]
    bounds: Option<DistributionBounds>,
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
    name: String,
    layout: GroupLayout,
    layers: Vec<Layer>,
}

#[derive(Deserialize)]
struct Template {
    size: Size,
    background: String,
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

impl GroupPosition {
    fn get_aligned_position(&self, content_size: (u32, u32), container_size: (u32, u32)) -> (u32, u32) {
        let x = match self.align {
            Align::Left => self.x,
            Align::Center => self.x + (container_size.0.saturating_sub(content_size.0)) / 2,
            Align::Right => self.x + container_size.0.saturating_sub(content_size.0),
        };
        (x, self.y)
    }
}

impl Position {
    fn get_aligned_position(&self, _content_size: (u32, u32), _container_size: (u32, u32)) -> (u32, u32) {
        // For now, we'll just return the absolute position
        // Relative positioning will be handled at a higher level when we have access to all layer information
        (self.x, self.y)
    }
}

impl Group {
    fn calculate_positions(&self, layers_info: &[(LayerDimensions, &LayerInfo)]) -> Vec<Position> {
        let mut positions = Vec::new();
        let mut current_x = self.layout.position.x;
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

        // Apply group alignment
        if let Some(dist_config) = &self.layout.distribution {
            if let Some(bounds) = &dist_config.bounds {
                let (x, _y) = self.layout.position.get_aligned_position(
                    (total_width, total_height),
                    (bounds.width, bounds.height)
                );
                current_x = x;
                // Keep current_y as is since vertical alignment is handled by distribution
            }
        } else {
            let (x, _y) = self.layout.position.get_aligned_position(
                (total_width, total_height),
                (total_width, total_height) // Use content size as container size when no bounds
            );
            current_x = x;
            // Keep current_y as is for consistency with the original layout
        }

        match self.layout.layout_type {
            LayoutType::Grid => {
                if let Some(dist_config) = &self.layout.distribution {
                    // Calculate grid dimensions
                    let columns = self.layout.columns as usize;
                    let rows = (layers_info.len() + columns - 1) / columns;
                    
                    // Get distribution bounds or calculate from content
                    let bounds = dist_config.bounds.as_ref().map(|b| (b.width, b.height)).unwrap_or_else(|| {
                        let total_width: u32 = layers_info.iter()
                            .take(columns)
                            .map(|(dims, _)| dims.width)
                            .sum();
                        let total_height: u32 = layers_info.iter()
                            .step_by(columns)
                            .take(rows)
                            .map(|(dims, _)| dims.height)
                            .sum();
                        (total_width, total_height)
                    });

                    // Calculate spacing
                    let h_spacing = match dist_config.horizontal {
                        Some(Distribution::SpaceBetween) => {
                            let content_width: u32 = layers_info.iter()
                                .take(columns)
                                .map(|(dims, _)| dims.width)
                                .sum();
                            if columns > 1 {
                                (bounds.0.saturating_sub(content_width)) / (columns as u32 - 1)
                            } else {
                                0
                            }
                        },
                        Some(Distribution::SpaceAround) => {
                            let content_width: u32 = layers_info.iter()
                                .take(columns)
                                .map(|(dims, _)| dims.width)
                                .sum();
                            bounds.0.saturating_sub(content_width) / (columns as u32 + 1)
                        },
                        Some(Distribution::SpaceEvenly) => {
                            let content_width: u32 = layers_info.iter()
                                .take(columns)
                                .map(|(dims, _)| dims.width)
                                .sum();
                            bounds.0.saturating_sub(content_width) / ((columns + 1) as u32)
                        },
                        None => self.layout.spacing,
                    };

                    let v_spacing = match dist_config.vertical {
                        Some(Distribution::SpaceBetween) => {
                            let content_height: u32 = layers_info.iter()
                                .step_by(columns)
                                .take(rows)
                                .map(|(dims, _)| dims.height)
                                .sum();
                            if rows > 1 {
                                (bounds.1.saturating_sub(content_height)) / (rows as u32 - 1)
                            } else {
                                0
                            }
                        },
                        Some(Distribution::SpaceAround) => {
                            let content_height: u32 = layers_info.iter()
                                .step_by(columns)
                                .take(rows)
                                .map(|(dims, _)| dims.height)
                                .sum();
                            bounds.1.saturating_sub(content_height) / (rows as u32 + 1)
                        },
                        Some(Distribution::SpaceEvenly) => {
                            let content_height: u32 = layers_info.iter()
                                .step_by(columns)
                                .take(rows)
                                .map(|(dims, _)| dims.height)
                                .sum();
                            bounds.1.saturating_sub(content_height) / ((rows + 1) as u32)
                        },
                        None => self.layout.spacing,
                    };

                    // Initial offset for space_around and space_evenly
                    let init_x = match dist_config.horizontal {
                        Some(Distribution::SpaceAround) | Some(Distribution::SpaceEvenly) => h_spacing,
                        _ => 0,
                    };
                    let init_y = match dist_config.vertical {
                        Some(Distribution::SpaceAround) | Some(Distribution::SpaceEvenly) => v_spacing,
                        _ => 0,
                    };

                    // Position elements
                    let mut x = current_x + init_x;
                    let mut y = current_y + init_y;
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
                            x = current_x + init_x;
                            y += dims.height + v_spacing;
                        } else {
                            x += dims.width + h_spacing;
                        }
                    }
                } else {
                    // Fallback to original grid layout
                    let columns = self.layout.columns as usize;
                    let spacing = self.layout.spacing;
                    
                    for (i, (dims, _)) in layers_info.iter().enumerate() {
                        let row = i / columns;
                        let col = i % columns;
                        
                        let x = current_x + col as u32 * (dims.width + spacing);
                        let y = current_y + row as u32 * (dims.height + spacing);
                        
                        positions.push(Position {
                            x,
                            y,
                            relative_to: RelativeTo::Canvas,
                        });
                    }
                }
            },
            LayoutType::Vertical => {
                let mut y_offset = 0;
                for (dims, _) in layers_info {
                    positions.push(Position {
                        x: current_x,
                        y: current_y + y_offset,
                        relative_to: RelativeTo::Canvas,
                    });
                    y_offset += dims.height + self.layout.spacing;
                }
            },
            LayoutType::Horizontal => {
                let mut x_offset = 0;
                for (dims, _) in layers_info {
                    positions.push(Position {
                        x: current_x + x_offset,
                        y: current_y,
                        relative_to: RelativeTo::Canvas,
                    });
                    x_offset += dims.width + self.layout.spacing;
                }
            },
        }

        // Apply individual layer alignments
        let mut relative_adjustments = Vec::new();
        for (i, pos) in positions.iter().enumerate() {
            if let RelativeTo::Layer(ref layer_name) = pos.relative_to {
                // Find the referenced layer's position
                if let Some((ref_idx, _)) = layers_info.iter()
                    .enumerate()
                    .find(|(_, (_, info))| info.name == *layer_name)
                {
                    relative_adjustments.push((i, ref_idx));
                }
            }
        }

        // Apply relative positioning
        for (target_idx, ref_idx) in relative_adjustments {
            let ref_pos = positions[ref_idx].clone();
            positions[target_idx].x = ref_pos.x;
            positions[target_idx].y = ref_pos.y;
        }

        positions
    }
}

#[derive(Deserialize)]
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
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Layer {
    Text(TextLayer),
    Image(ImageLayer),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create output directory if it doesn't exist
    std::fs::create_dir_all("output")?;

    // Load and parse the template
    let mut template_file = File::open("templates/example.json")?;
    let mut template_contents = String::new();
    template_file.read_to_string(&mut template_contents)?;
    let template: Template = serde_json::from_str(&template_contents)?;

    // Create canvas
    let mut canvas = RgbaImage::new(template.size.width, template.size.height);
    
    // Fill background with specified color
    let bg_color = parse_color(&template.background)?;
    for pixel in canvas.pixels_mut() {
        *pixel = Rgba([
            (bg_color.r * 255.0) as u8,
            (bg_color.g * 255.0) as u8,
            (bg_color.b * 255.0) as u8,
            (bg_color.a * 255.0) as u8,
        ]);
    }

    // Process each group
    for group in template.groups {
        // First pass: calculate dimensions for all layers in group
        let mut layer_info = Vec::new();
        for layer in &group.layers {
            let dims = layer.get_dimensions()?;
            let info = match layer {
                Layer::Text(text_layer) => &text_layer.info,
                Layer::Image(image_layer) => &image_layer.info,
            };
            layer_info.push((dims, info));
        }

        // Calculate positions for all layers in group
        let positions = group.calculate_positions(&layer_info);

        // Second pass: render layers with calculated positions
        for (layer, position) in group.layers.into_iter().zip(positions) {
            match layer {
                Layer::Text(mut text_layer) => {
                    text_layer.info.position = Some(position);
                    println!("Processing text layer: {}", text_layer.info.name);
                    text_layer.validate()?;

                    // Load font from system
                    let font = text_layer.font.load_font()?;
                    let text_color = parse_color(&text_layer.font.color)?;
                    let rgba_color = Rgba([
                        (text_color.r * 255.0) as u8,
                        (text_color.g * 255.0) as u8,
                        (text_color.b * 255.0) as u8,
                        (text_color.a * 255.0) as u8,
                    ]);
                    let scale = Scale::uniform(text_layer.font.size);
                    let v_metrics = font.v_metrics(scale);

                    // Calculate text dimensions
                    let text = text_layer.text.replace("{{name}}", "World");
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

                    let x_position = match text_layer.alignment {
                        TextAlignment::Center => text_layer.info.position.as_ref().map_or(0, |p| p.x.saturating_sub(text_width / 2)),
                        TextAlignment::Right => text_layer.info.position.as_ref().map_or(0, |p| p.x.saturating_sub(text_width)),
                        TextAlignment::Left => text_layer.info.position.as_ref().map_or(0, |p| p.x),
                    };

                    // Apply justification spacing
                    let justified_spacing = match text_layer.justification {
                        TextJustification::Justify => {
                            let words = text.split_whitespace().count();
                            if words > 1 {
                                Some((template.size.width - text_width) as f32 / (words - 1) as f32)
                            } else {
                                None
                            }
                        },
                        _ => None,
                    };

                    // Layout the text with justification if needed
                    let mut current_x = x_position as f32;
                    let y_position = text_layer.info.position.as_ref().map_or(0, |p| p.y);
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
                        if let Some(pos) = &text_layer.info.position {
                            text_layer.font.draw_decoration(
                                &mut canvas,
                                rgba_color,
                                current_x as u32,
                                pos.y,
                                word_width,
                                text_height,
                            );
                        }

                        // Update x position for next word
                        if i < words.len() - 1 {
                            current_x += word_width as f32 + if let Some(spacing) = justified_spacing {
                                spacing
                            } else {
                                scale.x // default space width
                            };
                        }
                    }
                },
                Layer::Image(mut image_layer) => {
                    image_layer.info.position = Some(position);
                    println!("Processing image layer: {}", image_layer.info.name);
                    image_layer.validate()?;

                    let mut overlay = image::open(&image_layer.source)?;
                    
                    // Apply scaling if needed
                    if image_layer.scale != 1.0 {
                        let new_width = (overlay.width() as f32 * image_layer.scale) as u32;
                        let new_height = (overlay.height() as f32 * image_layer.scale) as u32;
                        overlay = overlay.resize(new_width, new_height, image::imageops::FilterType::Lanczos3);
                    }

                    if let Some(pos) = &image_layer.info.position {
                        image::imageops::overlay(
                            &mut canvas,
                            &overlay,
                            pos.x as i64,
                            pos.y as i64,
                        );
                    }
                }
            }
        }
    }

    // Save the result
    canvas.save("output/result.png")?;
    println!("Image has been created successfully in output/result.png!");
    
    Ok(())
}

