macro_rules! workflows_count {
    ($workflows_arc:expr) => {
        $crate::common::utils::workflows_count(&$workflows_arc)
    };
}
pub(crate) use workflows_count;

macro_rules! move_selection {
    ($self:expr, Previous) => {
        $crate::common::utils::move_selection_previous(&mut $self)
    };
    ($self:expr, Previous, $max:expr) => {
        $crate::common::utils::move_selection_previous(&mut $self)
    };
    ($self:expr, Next, $max:expr) => {
        $crate::common::utils::move_selection_next(&mut $self, $max)
    };
}
pub(crate) use move_selection;
