//! 贪吃蛇游戏逻辑模块

use alloc::vec::Vec;
use alloc::boxed::Box;
use defmt::Format;

// 随机数种子（由 main.rs 设置）
static mut RANDOM_SEED: u16 = 0;

/// 设置随机数种子
pub fn set_random_seed(seed: u16) {
    unsafe {
        RANDOM_SEED = seed;
    }
}

/// 获取伪随机数
fn random() -> u16 {
    unsafe {
        // 简单的线性同余生成器
        let seed = RANDOM_SEED;
        let new_seed = seed.wrapping_mul(25173).wrapping_add(13849);
        RANDOM_SEED = new_seed;
        new_seed
    }
}

// 游戏常量
pub const GRID_SIZE: u16 = 10;          // 每格大小 10x10 像素
pub const GRID_COUNT: u16 = 24;         // 24x24 格子
pub const SCREEN_SIZE: u16 = 240;       // 屏幕大小 240x240

/// 方向枚举
#[derive(Clone, Copy, PartialEq, Format)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    /// 获取相反方向
    pub fn opposite(&self) -> Direction {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
}

/// 位置坐标
#[derive(Clone, Copy, PartialEq)]
pub struct Position {
    pub x: u16,
    pub y: u16,
}

/// 游戏状态
pub struct Game {
    pub snake: Vec<Position>,
    pub direction: Direction,
    next_direction: Direction,
    pub food: Position,
    pub score: u16,
    pub game_over: bool,
}

impl Game {
    /// 创建新游戏
    pub fn new() -> Box<Self> {
        // 初始化蛇在屏幕中央，长度为3
        let start_x = GRID_COUNT / 2;
        let start_y = GRID_COUNT / 2;
        let snake = alloc::vec![
            Position { x: start_x, y: start_y },
            Position { x: start_x - 1, y: start_y },
            Position { x: start_x - 2, y: start_y },
        ];
        
        let mut game = Box::new(Game {
            snake,
            direction: Direction::Right,
            next_direction: Direction::Right,
            food: Position { x: 0, y: 0 },
            score: 0,
            game_over: false,
        });
        game.spawn_food();
        game
    }

    /// 生成食物（不在蛇身上）
    fn spawn_food(&mut self) {
        // 使用伪随机数生成位置
        loop {
            let new_food = Position {
                x: random() % GRID_COUNT,
                y: random() % GRID_COUNT,
            };
            
            // 检查是否在蛇身上
            let on_snake = self.snake.iter().any(|p| p.x == new_food.x && p.y == new_food.y);
            if !on_snake {
                self.food = new_food;
                break;
            }
        }
    }

    /// 设置方向（防止180度转向）
    pub fn set_direction(&mut self, dir: Direction) {
        // defmt::println!("set_direction: 当前方向={}, 请求方向={}", self.direction, dir);
        // 检查是否与当前移动方向相反
        if dir != self.direction.opposite() {
            self.next_direction = dir;
            self.direction = dir;
            defmt::println!("方向已更新为: {}", self.direction);
        } else {
            defmt::println!("方向被拒绝（180度转向）");
        }
    }

    /// 游戏更新
    pub fn update(&mut self) {
        if self.game_over {
            return;
        }

        // 计算新头部位置
        let head = self.snake.first().unwrap();
        // defmt::println!("更新: 方向={}, 蛇头位置=({},{})", self.direction, head.x, head.y);
        
        let new_head = match self.direction {
            Direction::Up => Position { x: head.x, y: head.y.wrapping_sub(1) },
            Direction::Down => Position { x: head.x, y: head.y.saturating_add(1) },
            Direction::Left => Position { x: head.x.wrapping_sub(1), y: head.y },
            Direction::Right => Position { x: head.x.saturating_add(1), y: head.y },
        };
        

        // 检查撞墙
        if new_head.x >= GRID_COUNT || new_head.y >= GRID_COUNT {
            self.game_over = true;
            return;
        }

        // 检查撞自己
        if self.snake.iter().any(|p| p.x == new_head.x && p.y == new_head.y) {
            self.game_over = true;
            return;
        }

        // 移动蛇
        self.snake.insert(0, new_head);

        // 检查吃食物
        if new_head.x == self.food.x && new_head.y == self.food.y {
            self.score += 1;
            self.spawn_food();
        } else {
            self.snake.pop();
        }
    }

    /// 重置游戏
    pub fn reset(&mut self) {
        let new_game = Self::new();
        *self = *new_game;
    }
}
