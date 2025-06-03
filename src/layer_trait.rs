pub trait SourceLayer {
    fn name(&self) -> &str;
    fn content(&self) -> &str;
    fn bounds(&self) -> Option<(f64, f64, f64, f64)>;
    fn font_name(&self) -> Option<&str>;
    fn color(&self) -> Option<(u8, u8, u8)>;
} 