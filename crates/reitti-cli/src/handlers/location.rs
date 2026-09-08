mod schema;
use super::HandlerContext;
use crate::{command::LocationListArgs, error::AppError, output::CommandOutput};
use reitti_core::Language;

pub fn execute(
    context: HandlerContext<'_>,
    args: LocationListArgs,
    language: Language,
) -> Result<CommandOutput, AppError> {
    let _ = (context, args, language);
    Err(AppError::feature_incomplete("location list"))
}
pub(crate) use schema::schema;
