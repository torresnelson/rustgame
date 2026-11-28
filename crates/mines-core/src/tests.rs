use super::*;

fn rng() -> fastrand::Rng {
    fastrand::Rng::with_seed(9)
}

const TINY: Difficulty = Difficulty {
    name: "TINY",
    columns: 4,
    rows: 3,
    mines: 2,
};

/// A 4x3 field with mines at (3, 0) and (3, 2):
///
/// ```text
/// 0 0 1 *
/// 0 0 2 2
/// 0 0 1 *
/// ```
fn field() -> Field {
    let mut field = Field::new(&TINY);
    field.lay_mines_at(&[(3, 0), (3, 2)]);
    field
}

#[test]
fn counts_number_the_touching_mines() {
    let field = field();
    assert_eq!(field.count((0, 0)), 0);
    assert_eq!(field.count((2, 0)), 1);
    assert_eq!(field.count((2, 1)), 2);
    assert_eq!(field.count((3, 1)), 2);
    assert_eq!(field.neighbours((0, 0)).len(), 3);
    assert_eq!(field.neighbours((1, 1)).len(), 8);
}

#[test]
fn opening_a_zero_floods_to_the_numbers() {
    let mut field = field();
    assert!(field.open((0, 0), &mut rng()));
    for y in 0..3 {
        for x in 0..3 {
            assert_eq!(field.cover((x, y)), Cover::Open, "({x}, {y})");
        }
    }
    assert_eq!(field.cover((3, 1)), Cover::Hidden);
    assert_eq!(field.status, Status::Playing);
}

#[test]
fn opening_every_safe_cell_wins_and_flags_the_mines() {
    let mut field = field();
    field.open((0, 0), &mut rng());
    field.open((3, 1), &mut rng());
    assert_eq!(field.status, Status::Won);
    assert_eq!(field.cover((3, 0)), Cover::Flagged);
    assert_eq!(field.mines_left(), 0);
}

#[test]
fn opening_a_mine_loses() {
    let mut field = field();
    field.open((3, 0), &mut rng());
    assert_eq!(field.status, Status::Lost);
    assert_eq!(field.exploded, Some((3, 0)));
    // Nothing more happens after the end.
    assert!(!field.open((0, 0), &mut rng()));
    assert!(!field.toggle_flag((1, 1)));
}

#[test]
fn flags_block_opening_and_toggle_off() {
    let mut field = field();
    assert!(field.toggle_flag((3, 0)));
    assert_eq!(field.mines_left(), 1);
    assert!(!field.open((3, 0), &mut rng()));
    assert_eq!(field.status, Status::Playing);
    assert!(field.toggle_flag((3, 0)));
    assert_eq!(field.cover((3, 0)), Cover::Hidden);
}

#[test]
fn open_cells_cannot_be_flagged() {
    let mut field = field();
    field.open((0, 0), &mut rng());
    assert!(!field.toggle_flag((0, 0)));
}

#[test]
fn a_chord_opens_the_rest_when_the_flags_match() {
    let mut field = field();
    field.open((2, 0), &mut rng());
    // No flag yet: the chord does nothing.
    assert!(!field.open((2, 0), &mut rng()));
    field.toggle_flag((3, 0));
    assert!(field.open((2, 0), &mut rng()));
    assert_eq!(field.cover((3, 1)), Cover::Open);
    assert_eq!(field.cover((1, 0)), Cover::Open);
    // (1, 0) was a zero, so the flood opened the rest: every safe cell is open.
    assert_eq!(field.status, Status::Won);
}

#[test]
fn a_chord_with_a_wrong_flag_sets_off_a_mine() {
    let mut field = field();
    field.open((2, 0), &mut rng());
    field.toggle_flag((3, 1)); // wrong: (3, 0) is the mine
    field.open((2, 0), &mut rng());
    assert_eq!(field.status, Status::Lost);
    assert_eq!(field.exploded, Some((3, 0)));
}

#[test]
fn the_first_opening_is_always_safe_and_lays_every_mine() {
    for seed in 0..50 {
        let mut rng = fastrand::Rng::with_seed(seed);
        let easy = DIFFICULTIES[0];
        let mut field = Field::new(&easy);
        assert!(!field.laid);
        field.open((4, 4), &mut rng);
        assert!(field.laid);
        assert_ne!(field.status, Status::Lost);
        assert_eq!(field.count((4, 4)), 0, "the first cell opens an area");
        let mines = field.cells().filter(|&c| field.is_mine(c)).count();
        assert_eq!(mines, easy.mines);
    }
}

#[test]
fn a_crowded_field_still_keeps_the_first_cell_safe() {
    let crowded = Difficulty {
        name: "CROWDED",
        columns: 3,
        rows: 3,
        mines: 8,
    };
    let mut field = Field::new(&crowded);
    field.open((1, 1), &mut rng());
    assert!(!field.is_mine((1, 1)));
    assert_eq!(field.count((1, 1)), 8);
}

#[test]
fn difficulties_fit_their_mines() {
    for d in DIFFICULTIES {
        assert!(d.mines + 9 <= (d.columns * d.rows) as usize, "{}", d.name);
    }
}
