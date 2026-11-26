use anyhow::Result;

use crate::common::print_struct_docs;
use utils::logger::print_lines;

pub(crate) fn explain_config() -> Result<()> {
    print_struct_docs!(config::Config)?;
    print_lines(1);
    print_struct_docs!(config::HexColorsConfig, 2)?;
    print_lines(1);
    print_struct_docs!(config::BrowserConfig, 2)?;
    print_lines(1);
    print_struct_docs!(config::AgentConfig, 2)?;
    print_lines(1);

    Ok(())
}
