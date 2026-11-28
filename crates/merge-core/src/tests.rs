use super::*;

fn rng() -> fastrand::Rng {
    fastrand::Rng::with_seed(5)
}

#[test]
fn values_are_powers_of_two() {
    assert_eq!(value(0), 0);
    assert_eq!(value(1), 2);
    assert_eq!(value(GOAL), 2048);
}

#[test]
fn slide_line_packs_and_merges_once() {
    // (line, slid, points), as exponents: 1 = 2, 2 = 4, 3 = 8.
    let cases: [([u8; 4], [u8; 4], u32); 7] = [
        ([0, 0, 0, 0], [0, 0, 0, 0], 0),
        ([0, 1, 0, 2], [1, 2, 0, 0], 0),
        ([1, 1, 0, 0], [2, 0, 0, 0], 4),
        ([1, 0, 0, 1], [2, 0, 0, 0], 4),
        // 2 2 2 2 -> 4 4, not 8.
        ([1, 1, 1, 1], [2, 2, 0, 0], 8),
        // 4 2 2 -> 4 4: the new 4 does not merge again this move.
        ([2, 1, 1, 0], [2, 2, 0, 0], 4),
        // 2 2 2 -> 4 2: the front pair merges first.
        ([1, 1, 1, 0], [2, 1, 0, 0], 4),
    ];
    for (line, slid, points) in cases {
        assert_eq!(slide_line(line), (slid, points), "{line:?}");
    }
}

#[test]
fn slide_moves_every_line_the_right_way() {
    let board: Board = [[1, 0, 0, 1], [0, 0, 0, 0], [0, 2, 0, 0], [0, 2, 0, 0]];
    let (left, points) = slide(&board, Direction::Left);
    assert_eq!(left[0], [2, 0, 0, 0]);
    assert_eq!(points, 4);
    let (right, _) = slide(&board, Direction::Right);
    assert_eq!(right[0], [0, 0, 0, 2]);
    assert_eq!(right[2], [0, 0, 0, 2]);
    let (up, points) = slide(&board, Direction::Up);
    assert_eq!(up[0], [1, 3, 0, 1]);
    assert_eq!(points, 8);
    let (down, _) = slide(&board, Direction::Down);
    assert_eq!(down[3], [1, 3, 0, 1]);
    assert_eq!(down[0], [0, 0, 0, 0]);
}

#[test]
fn a_new_puzzle_has_two_tiles() {
    let puzzle = Puzzle::new(&mut rng());
    assert_eq!(empty_cells(&puzzle.board).len(), SIZE * SIZE - 2);
    assert!(puzzle.board.iter().flatten().all(|&t| t <= 2));
    assert_eq!(puzzle.score, 0);
}

#[test]
fn a_move_that_changes_nothing_adds_no_tile() {
    let mut puzzle = Puzzle::new(&mut rng());
    puzzle.board = [[1, 0, 0, 0], [0; 4], [0; 4], [0; 4]];
    assert!(!puzzle.play(Direction::Left, &mut rng()));
    assert!(!puzzle.play(Direction::Up, &mut rng()));
    assert_eq!(empty_cells(&puzzle.board).len(), 15);
    assert!(puzzle.play(Direction::Right, &mut rng()));
    assert_eq!(empty_cells(&puzzle.board).len(), 14);
    assert_eq!(puzzle.board[0][3], 1);
    let (r, c) = puzzle.spawned.unwrap();
    assert_ne!(puzzle.board[r][c], 0);
}

#[test]
fn merging_scores_the_new_tile() {
    let mut puzzle = Puzzle::new(&mut rng());
    puzzle.board = [[3, 3, 2, 2], [0; 4], [0; 4], [0; 4]];
    puzzle.play(Direction::Left, &mut rng());
    assert_eq!(puzzle.score, 16 + 8);
    assert_eq!(&puzzle.board[0][..2], &[4, 3]);
}

#[test]
fn spawned_tiles_are_mostly_twos() {
    let mut rng = rng();
    let mut fours = 0;
    for _ in 0..1000 {
        let mut board = [[0; SIZE]; SIZE];
        let (r, c) = spawn(&mut board, &mut rng).unwrap();
        assert!(board[r][c] == 1 || board[r][c] == 2);
        fours += usize::from(board[r][c] == 2);
    }
    assert!((50..150).contains(&fours), "{fours} fours");
}

#[test]
fn a_full_board_without_pairs_is_over() {
    let stuck: Board = [[1, 2, 1, 2], [2, 1, 2, 1], [1, 2, 1, 2], [2, 1, 2, 1]];
    assert!(!can_move(&stuck));
    let mut board = stuck;
    assert_eq!(spawn(&mut board, &mut rng()), None);
    let mut pair = stuck;
    pair[3][3] = 2; // next to the 2 above it
    assert!(can_move(&pair));
}

#[test]
fn reaching_2048_is_the_goal() {
    let mut puzzle = Puzzle::new(&mut rng());
    puzzle.board = [[10, 10, 0, 0], [0; 4], [0; 4], [0; 4]];
    assert!(!puzzle.reached_goal());
    puzzle.play(Direction::Left, &mut rng());
    assert!(puzzle.reached_goal());
    assert_eq!(highest(&puzzle.board), GOAL);
}
