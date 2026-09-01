/// A 33-bit MPEG presentation timestamp measured on the 90 kHz clock.
///
/// Keep this distinct from millisecond media/project times: crossing that
/// boundary must be an explicit conversion so a raw transport timestamp is
/// not accidentally treated as a UI or archive time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pts90k(u64);

impl Pts90k {
    pub const MAX: u64 = (1_u64 << 33) - 1;

    pub fn new(ticks: u64) -> Option<Self> {
        (ticks <= Self::MAX).then_some(Self(ticks))
    }

    pub const fn ticks(self) -> u64 {
        self.0
    }

    pub fn to_millis(self) -> i64 {
        i64::try_from(self.0 / 90).expect("33-bit PTS always fits in i64 milliseconds")
    }
}

#[cfg(test)]
mod tests {
    use super::Pts90k;

    #[test]
    fn pts90k_has_a_bounded_transport_domain() {
        assert_eq!(Pts90k::new(90_000).map(Pts90k::to_millis), Some(1_000));
        assert_eq!(
            Pts90k::new(Pts90k::MAX).map(Pts90k::ticks),
            Some(Pts90k::MAX)
        );
        assert_eq!(Pts90k::new(Pts90k::MAX + 1), None);
    }
}
