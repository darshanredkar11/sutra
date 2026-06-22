#[derive(Debug, Clone)]
pub struct Config {
    pub memory: bool,
    pub complexity: bool,
    pub gaps: bool,
    pub computation: bool,
    pub min_severity: SeverityFilter,
    pub output_format: OutputFormat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeverityFilter {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputFormat {
    Pretty,
    Json,
    Quiet,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            memory: true,
            complexity: true,
            gaps: true,
            computation: true,
            min_severity: SeverityFilter::Info,
            output_format: OutputFormat::Pretty,
        }
    }
}

impl Config {
    pub fn to_map(&self) -> std::collections::HashMap<String, bool> {
        let mut m = std::collections::HashMap::new();
        m.insert("memory".to_string(), self.memory);
        m.insert("complexity".to_string(), self.complexity);
        m.insert("gaps".to_string(), self.gaps);
        m.insert("computation".to_string(), self.computation);
        m
    }
}
