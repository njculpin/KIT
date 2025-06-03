use std::error::Error;
use std::fs::File;
use std::io::Read;
use image::{RgbaImage, Rgba};
use serde::Deserialize;
use crate::layer_trait::SourceLayer;

#[derive(Debug, Clone)]
pub struct AiLayer {
    pub name: String,
    pub content: String,
    pub bounds: Option<(f64, f64, f64, f64)>, // x1, y1, x2, y2
    pub font_name: Option<String>,
    pub color: Option<(u8, u8, u8)>, // RGB
}

impl SourceLayer for AiLayer {
    fn name(&self) -> &str {
        &self.name
    }

    fn content(&self) -> &str {
        &self.content
    }

    fn bounds(&self) -> Option<(f64, f64, f64, f64)> {
        self.bounds
    }

    fn font_name(&self) -> Option<&str> {
        self.font_name.as_deref()
    }

    fn color(&self) -> Option<(u8, u8, u8)> {
        self.color
    }
}

#[derive(Deserialize, Debug)]
struct DesignMetafield {
    namespace: String,
    value: String,
}

#[derive(Deserialize, Debug)]
struct AiFileData {
    design_metafields: Vec<DesignMetafield>,
}

pub struct AiData {
    layer_names: Vec<String>,
}

impl AiData {
    pub fn new(json_path: &str, _source_image_path: Option<&str>) -> Result<Self, Box<dyn Error>> {
        println!("Loading AI file from path: {}", json_path);
        let mut file = File::open(json_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        println!("Raw JSON content: {}", contents);
        let ai_data: AiFileData = serde_json::from_str(&contents)?;
        println!("Parsed design_metafields: {:?}", ai_data.design_metafields);
        let layer_names = ai_data.design_metafields
            .into_iter()
            .filter(|m| m.namespace == "layer")
            .map(|m| m.value)
            .collect();
        Ok(Self { layer_names })
    }

    pub fn get_layer_by_name(&self, name: &str) -> Option<&dyn SourceLayer> {
        if self.layer_names.iter().any(|n| n == name) {
            // Return a dummy AiLayer with the correct name; content will be injected from the template
            // Use a static dummy so the reference is valid
            thread_local! {
                static DUMMY: AiLayer = AiLayer {
                    name: String::new(),
                    content: String::new(),
                    bounds: Some((0.1, 0.1, 0.9, 0.9)),
                    font_name: Some("Arial".to_string()),
                    color: Some((0, 0, 0)),
                };
            }
            DUMMY.with(|dummy| unsafe {
                let mut_ref = &*(dummy as *const AiLayer);
                // This is safe because we only use the name field for matching
                Some(mut_ref as &dyn SourceLayer)
            })
        } else {
            None
        }
    }
} 