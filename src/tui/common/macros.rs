macro_rules! workflows_count {
    ($workflows_arc:expr) => {
        $crate::tui::common::utils::workflows_count(&$workflows_arc)
    };
}
pub(crate) use workflows_count;

macro_rules! move_selection {
    ($self:expr, Previous) => {
        $crate::tui::common::utils::move_selection_previous(&mut $self)
    };
    ($self:expr, Previous, $max:expr) => {
        $crate::tui::common::utils::move_selection_previous(&mut $self)
    };
    ($self:expr, Next, $max:expr) => {
        $crate::tui::common::utils::move_selection_next(&mut $self, $max)
    };
}
pub(crate) use move_selection;
