mod schema;
use super::HandlerContext;
use crate::{command::JourneyListArgs, error::AppError, output::CommandOutput};
use reitti_core::Language;

pub fn execute(
    context: HandlerContext<'_>,
    args: JourneyListArgs,
    language: Language,
) -> Result<CommandOutput, AppError> {
    let _ = (context, args, language);
    Err(AppError::feature_incomplete("journey list"))
}
pub(crate) use schema::schema;
