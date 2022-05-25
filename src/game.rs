use std::{fmt::Display, net::IpAddr, sync::RwLock, time::Duration};

use crate::rate_limiter::RateLimiter;
const MAX_COLOR: u8 = 31;

#[derive(Debug, Clone)]
pub struct GameConfig {
    pub width: u32,
    pub height: u32,
    pub tile_wait_time: Duration,
}
#[derive(Debug)]
pub struct Game {
    width: u32,
    height: u32,
    board: RwLock<Vec<u8>>,
    rate_limiter: RateLimiter<IpAddr>,
}
impl Game {
    pub fn new(config: GameConfig) -> Self {
        Self {
            width: config.width,
            height: config.height,
            board: RwLock::new(vec![31; (config.width * config.height) as usize]),
            rate_limiter: RateLimiter::new(config.tile_wait_time),
        }
    }
    pub fn get_width(&self) -> u32 {
        self.width
    }
    pub fn get_height(&self) -> u32 {
        self.height
    }

    pub fn snapshot(&self) -> Vec<u8> {
        self.board.read().unwrap().clone()
    }

    pub fn load(board: impl Into<Vec<u8>>, height: GameConfig) -> Result<Game, LoadError> {
        let board = board.into();
        if board.len() != (height.width * height.height) as usize {
            return Err(LoadError::InvalidBoardSize);
        }
        if board.iter().any(|&x| x > 31) {
            return Err(LoadError::InvalidBoardData);
        }
        Ok(Game {
            width: height.width,
            height: height.height,
            board: RwLock::new(board),
            rate_limiter: RateLimiter::new(height.tile_wait_time),
        })
    }
    pub fn get_tile_color(&self, idx: u32) -> u8 {
        self.board.read().unwrap()[(idx) as usize]
    }
    pub fn set_tile(&self, ip: IpAddr, index: u32, color: u8) -> Result<(), SetTileError> {
        if !self.rate_limiter.is_free(&ip) {
            return Err(SetTileError::RateLimited);
        }
        if color > MAX_COLOR {
            return Err(SetTileError::InvalidColor);
        }
        if index >= self.width * self.height {
            return Err(SetTileError::OutOfBounds);
        }

        {
            let mut board = self.board.write().unwrap();
            board[index as usize] = color;
        }

        self.rate_limiter.mark_as_limited(ip);
        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum LoadError {
    InvalidBoardSize,
    InvalidBoardData,
}
impl Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::InvalidBoardSize => write!(f, "Invalid board size"),
            LoadError::InvalidBoardData => write!(f, "Invalid board data"),
        }
    }
}
impl std::error::Error for LoadError {}

#[derive(Debug, Eq, PartialEq)]
pub enum SetTileError {
    OutOfBounds,
    InvalidColor,
    RateLimited,
}

impl Display for SetTileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SetTileError::OutOfBounds => write!(f, "Out of bounds"),
            SetTileError::InvalidColor => write!(f, "Invalid color"),
            SetTileError::RateLimited => write!(f, "Rate limited"),
        }
    }
}
impl std::error::Error for SetTileError {}
#[cfg(test)]
mod tests {
    use std::vec;

    use super::*;
    const WIDTH: u32 = 10;
    const HEIGHT: u32 = 10;
    const TEST_IP: IpAddr = IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1));
    const RATE_LIMIT_TIME: Duration = Duration::from_millis(100);
    #[fixture]
    fn game() -> Game {
        Game::new(GameConfig {
            width: WIDTH,
            height: HEIGHT,
            tile_wait_time: RATE_LIMIT_TIME,
        })
    }

    #[rstest]
    #[test]
    fn width_and_height_of_the_game_must_match_inputs(game: Game) {
        assert_eq!(game.get_width(), WIDTH);
        assert_eq!(game.get_height(), HEIGHT);
    }
    #[rstest]
    #[test]
    fn initial_game_snapshot_must_be_all_white(game: Game) {
        let board = game.snapshot();
        for cell in board.into_iter() {
            assert_eq!(cell, 31);
        }
    }

    mod load {
        use super::*;
        #[fixture]
        fn load_conf() -> GameConfig {
            GameConfig {
                width: WIDTH,
                height: HEIGHT,
                tile_wait_time: RATE_LIMIT_TIME,
            }
        }
        #[rstest]
        #[test]
        fn initially_loaded_board_must_exists_as_is(load_conf: GameConfig) {
            let loaded_board = vec![24; HEIGHT as usize * WIDTH as usize];
            let game = Game::load(loaded_board.clone(), load_conf).unwrap();
            let snapshot = game.snapshot();
            assert_eq!(snapshot, loaded_board);
        }

        #[rstest]
        #[test]
        fn it_must_return_error_if_width_and_height_do_not_match(load_conf: GameConfig) {
            let loaded_board = vec![24; HEIGHT as usize * WIDTH as usize + 1];
            let err =
                Game::load(loaded_board, load_conf).expect_err("expected load to return error");
            assert_eq!(err, LoadError::InvalidBoardSize);
        }

        #[rstest]
        #[test]
        fn it_must_return_error_if_loaded_board_is_invalid(load_conf: GameConfig) {
            let loaded_board = vec![45; HEIGHT as usize * WIDTH as usize];
            let err =
                Game::load(loaded_board, load_conf).expect_err("expected load to return error");
            assert_eq!(err, LoadError::InvalidBoardData);
        }
    }

    mod set_tile {
        use super::*;

        #[rstest]
        fn it_must_return_error_if_index_is_out_of_range(game: Game) {
            let err = game
                .set_tile(TEST_IP, WIDTH * HEIGHT, 0)
                .expect_err("expected set_tile to return error");
            assert_eq!(err, SetTileError::OutOfBounds);
        }

        #[rstest]
        fn it_must_return_error_if_color_is_not_valid(game: Game) {
            let err = game
                .set_tile(TEST_IP, 0, MAX_COLOR + 1)
                .expect_err("expected set_tile to return error");
            assert_eq!(err, SetTileError::InvalidColor);
        }

        mod rate_limit {
            use super::*;
            #[rstest]
            fn it_must_not_be_rate_limited_for_first_time(game: Game) {
                assert!(game.set_tile(TEST_IP, 0, 0).is_ok());
            }

            #[rstest]
            fn it_must_return_error_if_ip_is_marked_as_rate_limited(game: Game) {
                game.set_tile(TEST_IP, 0, 0).unwrap();
                let err = game
                    .set_tile(TEST_IP, 3, 1)
                    .expect_err("expected set_tile to return error");

                assert_eq!(err, SetTileError::RateLimited);
            }

            #[rstest]
            fn multiple_set_tiles_with_different_ips_should_not_rate_limit_each_other(game: Game) {
                game.set_tile(TEST_IP, 0, 0).unwrap();
                assert!(game
                    .set_tile(IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 2)), 0, 0)
                    .is_ok());
            }
        }

        #[rstest]
        fn set_tile_should_effect_snapshot(game: Game) {
            game.set_tile(TEST_IP, 55, 30).unwrap();
            let new_snapshot = game.snapshot();
            assert_eq!(new_snapshot[55], 30);
        }
    }
}
