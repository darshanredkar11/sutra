use crate::error::SutraResult;
use sutra_schema::v1::{AnalysisResult, AnalyzeRequest};

pub trait AnalysisEngine: Send + Sync {
    fn name(&self) -> &'static str;
    fn analyze(&self, request: &AnalyzeRequest) -> SutraResult<AnalysisResult>;
}
