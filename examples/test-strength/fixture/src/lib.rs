/// Only counts strictly below ten are allowed: ten must be rejected.
pub fn allowed(count: u32) -> bool {
    count < 10
}
