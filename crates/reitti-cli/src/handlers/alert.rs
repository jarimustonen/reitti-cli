mod schema;
use super::HandlerContext;
use crate::{command::AlertListArgs, error::AppError, output::CommandOutput};
use reitti_core::Language;

pub fn execute(
    context: HandlerContext<'_>,
    args: AlertListArgs,
    language: Language,
) -> Result<CommandOutput, AppError> {
    let _ = (context, args, language);
    Err(AppError::feature_incomplete("alert list"))
}
pub(crate) use schema::schema;
