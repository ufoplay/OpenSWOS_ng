use proptest::prelude::*;

use super::{NothingSaved, Rng, SAVE_STACK_LEN, SaveStackFull};
use crate::generator::Generator;

fn generator() -> impl Strategy<Value = Generator> {
    (any::<u8>(), any::<u8>(), any::<u8>()).prop_map(|(seed, xor_index, xor_key)| Generator {
        seed,
        xor_index,
        xor_key,
    })
}

fn rng(primary: Generator, secondary: Generator) -> Rng {
    Rng {
        primary,
        secondary,
        fixed: None,
        saved: Vec::new(),
    }
}

proptest! {
    #[test]
    fn each_generator_draws_alone(primary in generator(), secondary in generator()) {
        let mut both = rng(primary, secondary);
        let (mut first, mut second) = (primary, secondary);
        prop_assert_eq!(both.rand(), first.next_value());
        prop_assert_eq!(both.rand2(), second.next_value());
        prop_assert_eq!((both.primary, both.secondary), (first, second));
    }

    #[test]
    fn a_fixed_value_pins_both(value: u8, primary in generator(), secondary in generator()) {
        let mut both = rng(primary, secondary);
        both.fixed = Some(value);
        prop_assert_eq!(both.rand(), u16::from(value));
        prop_assert_eq!(both.rand2(), u16::from(value));
        prop_assert_eq!(both.random_word(), u16::from_le_bytes([value, value]));
        prop_assert_eq!((both.primary, both.secondary), (primary, secondary));
    }

    #[test]
    fn a_random_word_is_two_draws_low_first(secondary in generator()) {
        let mut both = rng(Generator::default(), secondary);
        let mut alone = secondary;
        let low = alone.next_value();
        let high = alone.next_value();
        let bytes = [low, high].map(|value| u8::try_from(value).expect("a byte"));
        prop_assert_eq!(both.random_word(), u16::from_le_bytes(bytes));
        prop_assert_eq!(both.secondary, alone);
    }

    #[test]
    fn randomize2_seeds_from_the_xor(value: u16, random_seed: u16, primary in generator()) {
        let mut both = rng(primary, Generator::default());
        both.randomize2(value, random_seed);
        let [seed, xor_index] = (value ^ random_seed).to_be_bytes();
        prop_assert_eq!(both.secondary, Generator::new(seed, xor_index));
        prop_assert_eq!(both.primary, primary);
    }
}

#[test]
fn the_saves_come_back_last_first() {
    let mut both = Rng::default();
    let states: Vec<Generator> = (1..SAVE_STACK_LEN)
        .map(|n| Generator::new(u8::try_from(n).expect("a few saves"), 0))
        .collect();
    for state in &states {
        both.secondary = *state;
        assert_eq!(both.save_rand2_state(), Ok(()));
    }
    assert_eq!(both.saved(), states.as_slice());
    for state in states.iter().rev() {
        both.secondary = Generator::default();
        assert_eq!(both.restore_rand2_state(), Ok(()));
        assert_eq!(both.secondary, *state);
    }
    assert_eq!(both.saved(), &[]);
}

#[test]
fn the_save_that_fills_the_stack_is_kept_and_halts() {
    let mut both = Rng::default();
    for _ in 1..SAVE_STACK_LEN {
        assert_eq!(both.save_rand2_state(), Ok(()));
    }
    both.secondary = Generator::new(1, 1);
    assert_eq!(both.save_rand2_state(), Err(SaveStackFull));
    assert_eq!(both.saved().len(), SAVE_STACK_LEN);
    assert_eq!(both.saved().last(), Some(&Generator::new(1, 1)));
    // With the stack full nothing more is saved.
    both.secondary = Generator::default();
    assert_eq!(both.save_rand2_state(), Err(SaveStackFull));
    assert_eq!(both.saved().last(), Some(&Generator::new(1, 1)));
}

#[test]
fn restoring_nothing_leaves_the_generator() {
    let state = Generator::new(1, 1);
    let mut both = rng(Generator::default(), state);
    assert_eq!(both.restore_rand2_state(), Err(NothingSaved));
    assert_eq!(both.secondary, state);
}
