mod schema;
use super::HandlerContext;
use crate::{
    command::{DepartureListArgs, StopListArgs},
    error::AppError,
    output::CommandOutput,
};
use reitti_core::Language;

pub fn execute_list(
    context: HandlerContext<'_>,
    args: StopListArgs,
    language: Language,
) -> Result<CommandOutput, AppError> {
    let _ = (context, args, language);
    Err(AppError::feature_incomplete("stop list"))
}
pub fn execute_departures(
    context: HandlerContext<'_>,
    args: DepartureListArgs,
    language: Language,
) -> Result<CommandOutput, AppError> {
    let _ = (context, args, language);
    Err(AppError::feature_incomplete("departure list"))
}
pub(crate) use schema::{departure_schema, stop_schema};
