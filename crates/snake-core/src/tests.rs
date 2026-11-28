use super::*;

fn rng() -> fastrand::Rng {
    fastrand::Rng::with_seed(3)
}

/// A snake with the given body (head first) heading right and no food.
fn snake(body: &[Cell]) -> Snake {
    let mut snake = Snake::new(&mut rng());
    snake.body = body.iter().copied().collect();
    snake.food = None;
    snake
}

#[test]
fn a_new_snake_is_centred_and_food_is_elsewhere() {
    let snake = Snake::new(&mut rng());
    assert_eq!(snake.body.len(), START_LENGTH as usize);
    assert_eq!(snake.head(), (COLUMNS / 2, ROWS / 2));
    assert_eq!(snake.heading, Direction::Right);
    let food = snake.food.unwrap();
    assert!(in_bounds(food));
    assert!(!snake.body.contains(&food));
}

#[test]
fn stepping_moves_the_whole_body() {
    let mut snake = snake(&[(5, 5), (4, 5), (3, 5)]);
    assert!(!snake.step(&mut rng()));
    assert_eq!(Vec::from(snake.body.clone()), [(6, 5), (5, 5), (4, 5)]);
}

#[test]
fn turns_ignore_reversing_and_repeats() {
    let mut snake = snake(&[(5, 5), (4, 5), (3, 5)]);
    snake.turn(Direction::Left);
    snake.turn(Direction::Right);
    assert_eq!(snake.next_heading(), Direction::Right);
    snake.step(&mut rng());
    assert_eq!(snake.head(), (6, 5));
}

#[test]
fn two_quick_turns_take_a_corner_one_step_each() {
    let mut snake = snake(&[(5, 5), (4, 5), (3, 5)]);
    snake.turn(Direction::Up);
    snake.turn(Direction::Left);
    snake.turn(Direction::Down); // a third is dropped
    snake.step(&mut rng());
    assert_eq!(snake.head(), (5, 4));
    snake.step(&mut rng());
    assert_eq!(snake.head(), (4, 4));
    assert_eq!(snake.heading, Direction::Left);
}

#[test]
fn eating_grows_scores_and_moves_the_food() {
    let mut snake = snake(&[(5, 5), (4, 5), (3, 5)]);
    snake.food = Some((6, 5));
    assert!(snake.step(&mut rng()));
    assert_eq!(snake.score, 1);
    assert_eq!(snake.body.len(), 4);
    let food = snake.food.unwrap();
    assert!(!snake.body.contains(&food));
    // The new cell stays: the tail did not move.
    assert_eq!(snake.body.back(), Some(&(3, 5)));
}

#[test]
fn hitting_a_wall_kills() {
    let mut snake = snake(&[(COLUMNS - 1, 0), (COLUMNS - 2, 0)]);
    snake.step(&mut rng());
    assert_eq!(snake.status, Status::Dead);
    assert_eq!(snake.head(), (COLUMNS - 1, 0));
    // A dead snake stays put.
    assert!(!snake.step(&mut rng()));
}

#[test]
fn hitting_itself_kills() {
    // A hook: the head turns down into its own body.
    let mut snake = snake(&[(5, 5), (4, 5), (4, 6), (5, 6), (6, 6)]);
    snake.turn(Direction::Down);
    snake.step(&mut rng());
    assert_eq!(snake.status, Status::Dead);
}

#[test]
fn chasing_the_tail_is_safe() {
    // A 2x2 loop: the head moves into the cell the tail is leaving.
    let mut snake = snake(&[(5, 5), (5, 6), (6, 6), (6, 5)]);
    snake.heading = Direction::Up;
    snake.turn(Direction::Right);
    snake.step(&mut rng());
    assert_eq!(snake.status, Status::Running);
    assert_eq!(snake.head(), (6, 5));
}

#[test]
fn eating_into_the_tail_cell_kills() {
    // The tail stays when the snake grows, so its cell is solid.
    let mut snake = snake(&[(5, 5), (5, 6), (6, 6), (6, 5)]);
    snake.heading = Direction::Up;
    snake.food = Some((6, 5));
    snake.turn(Direction::Right);
    snake.step(&mut rng());
    assert_eq!(snake.status, Status::Dead);
}

#[test]
fn filling_the_grid_wins() {
    // Snake along every row but the last free cell, which holds the food.
    let mut body: Vec<Cell> = Vec::new();
    for y in 0..ROWS {
        let row: Vec<Cell> = (0..COLUMNS).map(|x| (x, y)).collect();
        if y % 2 == 0 {
            body.extend(row.into_iter().rev());
        } else {
            body.extend(row);
        }
    }
    body.reverse(); // head at the end of the path
    let food = body.remove(0);
    let mut snake = snake(&body);
    snake.food = Some(food);
    let (hx, hy) = snake.head();
    snake.heading = match (food.0 - hx, food.1 - hy) {
        (1, 0) => Direction::Right,
        (-1, 0) => Direction::Left,
        (0, 1) => Direction::Down,
        _ => Direction::Up,
    };
    assert!(snake.step(&mut rng()));
    assert_eq!(snake.status, Status::Won);
    assert_eq!(snake.food, None);
}

#[test]
fn place_food_avoids_the_body_and_ends_when_full() {
    let body: VecDeque<Cell> = (0..COLUMNS).map(|x| (x, 0)).collect();
    let mut rng = rng();
    for _ in 0..50 {
        let food = place_food(&body, &mut rng).unwrap();
        assert!(food.1 > 0);
    }
    let full: VecDeque<Cell> = (0..ROWS)
        .flat_map(|y| (0..COLUMNS).map(move |x| (x, y)))
        .collect();
    assert_eq!(place_food(&full, &mut rng), None);
}

#[test]
fn steps_speed_up_with_meals_down_to_a_floor() {
    assert_eq!(step_ms(0), START_STEP_MS);
    assert_eq!(step_ms(5), START_STEP_MS - 5 * SPEEDUP_MS);
    assert_eq!(step_ms(1000), FASTEST_STEP_MS);
}
