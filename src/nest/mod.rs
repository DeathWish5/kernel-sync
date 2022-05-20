mod interrupt;

pub trait NestStrategy {
    fn push_off();
    fn pop_off();
}

pub const NO_IRQ_NEST: usize = 0;
pub const MOCK_NEST: usize = 1;

pub struct NoIrqNest;

impl NestStrategy for NoIrqNest {
    fn push_off() {
        interrupt::push_off();
    }
    fn pop_off() {
        interrupt::pop_off();
    }
}

pub struct MockNest;

impl NestStrategy for MockNest {
    fn push_off() {}
    fn pop_off() {}
}
