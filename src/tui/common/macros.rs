macro_rules! workflows_count {
    ($core:expr) => {
        $core.read().map(|core| core.workflows.len()).unwrap_or(0)
    };
}
pub(crate) use workflows_count;

macro_rules! move_selection {
    ($self:expr, Previous) => {
        macros::move_selection!($self, Previous, 0)
    };
    ($self:expr, Previous, $max:expr) => {{
        let selected = match $self.selected() {
            Some(current) => current.saturating_sub(1),
            None => 0,
        };
        $self.select(Some(selected));
    }};
    ($self:expr, Next, $max:expr) => {{
        let selected = match $self.selected() {
            Some(current) => {
                if current < $max {
                    current + 1
                } else {
                    current
                }
            }
            None => 0,
        };
        $self.select(Some(selected));
    }};
}
pub(crate) use move_selection;
