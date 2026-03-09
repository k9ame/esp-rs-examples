#![no_std]

extern crate alloc;

/// 游戏板宽度（格子数）
pub const BOARD_WIDTH: usize = 10;
/// 游戏板高度（格子数）
pub const BOARD_HEIGHT: usize = 20;
/// 每个格子的像素大小
pub const CELL_SIZE: u16 = 12;
/// 屏幕宽度（像素）
pub const SCREEN_WIDTH: u16 = 160;
/// 屏幕高度（像素）
pub const SCREEN_HEIGHT: u16 = 240;

/// 随机数种子
static mut RANDOM_SEED: u32 = 0;

/// 设置随机数种子
pub fn set_random_seed(seed: u16) {
    unsafe {
        RANDOM_SEED = seed as u32;
    }
}

/// 简单的伪随机数生成器
pub fn random() -> u16 {
    unsafe {
        RANDOM_SEED = RANDOM_SEED.wrapping_mul(1103515245).wrapping_add(12345);
        (RANDOM_SEED >> 16) as u16
    }
}

/// 获取范围内的随机数
pub fn random_range(min: u16, max: u16) -> u16 {
    let r = random();
    min + (r % (max - min))
}

/// 方块类型
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TetrominoType {
    I, // 长条
    O, // 方块
    T, // T形
    S, // S形
    Z, // Z形
    J, // J形
    L, // L形
}

impl TetrominoType {
    /// 获取所有方块类型
    pub const ALL: [TetrominoType; 7] = [
        TetrominoType::I,
        TetrominoType::O,
        TetrominoType::T,
        TetrominoType::S,
        TetrominoType::Z,
        TetrominoType::J,
        TetrominoType::L,
    ];

    /// 随机获取一个方块类型
    pub fn random() -> Self {
        let idx = random_range(0, 7) as usize;
        Self::ALL[idx]
    }

    /// 获取方块颜色 (R, G, B) - 565格式
    pub fn color(&self) -> (u8, u8, u8) {
        match self {
            TetrominoType::I => (0, 31, 31),    // 青色
            TetrominoType::O => (31, 31, 0),    // 黄色
            TetrominoType::T => (31, 0, 31),    // 紫色
            TetrominoType::S => (0, 31, 0),     // 绿色
            TetrominoType::Z => (31, 0, 0),     // 红色
            TetrominoType::J => (0, 0, 31),     // 蓝色
            TetrominoType::L => (31, 16, 0),    // 橙色
        }
    }
}

/// 方块形状定义
#[derive(Clone, Copy)]
pub struct Tetromino {
    pub kind: TetrominoType,
    pub x: i16,
    pub y: i16,
    pub rotation: u8, // 0-3 表示四种旋转状态
}

impl Tetromino {
    pub fn new(kind: TetrominoType) -> Self {
        Self {
            kind,
            x: (BOARD_WIDTH as i16 / 2) - 2,
            y: 0,
            rotation: 0,
        }
    }

    /// 获取当前旋转状态下的形状数据
    /// 返回4x4数组，true表示有方块
    pub fn shape(&self) -> [[bool; 4]; 4] {
        let base = match self.kind {
            TetrominoType::I => [
                [false, false, false, false],
                [true, true, true, true],
                [false, false, false, false],
                [false, false, false, false],
            ],
            TetrominoType::O => [
                [true, true, false, false],
                [true, true, false, false],
                [false, false, false, false],
                [false, false, false, false],
            ],
            TetrominoType::T => [
                [false, true, false, false],
                [true, true, true, false],
                [false, false, false, false],
                [false, false, false, false],
            ],
            TetrominoType::S => [
                [false, true, true, false],
                [true, true, false, false],
                [false, false, false, false],
                [false, false, false, false],
            ],
            TetrominoType::Z => [
                [true, true, false, false],
                [false, true, true, false],
                [false, false, false, false],
                [false, false, false, false],
            ],
            TetrominoType::J => [
                [true, false, false, false],
                [true, true, true, false],
                [false, false, false, false],
                [false, false, false, false],
            ],
            TetrominoType::L => [
                [false, false, true, false],
                [true, true, true, false],
                [false, false, false, false],
                [false, false, false, false],
            ],
        };

        // 根据旋转次数旋转形状
        let mut result = base;
        for _ in 0..self.rotation {
            result = Self::rotate_right(&result);
        }
        result
    }

    /// 顺时针旋转90度
    fn rotate_right(shape: &[[bool; 4]; 4]) -> [[bool; 4]; 4] {
        let mut result = [[false; 4]; 4];
        for y in 0..4 {
            for x in 0..4 {
                result[x][3 - y] = shape[y][x];
            }
        }
        result
    }

    /// 旋转方块
    pub fn rotate(&mut self) {
        self.rotation = (self.rotation + 1) % 4;
    }

    /// 向左移动
    pub fn move_left(&mut self) {
        self.x -= 1;
    }

    /// 向右移动
    pub fn move_right(&mut self) {
        self.x += 1;
    }

    /// 向下移动
    pub fn move_down(&mut self) {
        self.y += 1;
    }
}

/// 游戏状态
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    Playing,
    GameOver,
    Paused,
}

/// 俄罗斯方块游戏
pub struct TetrisGame {
    /// 游戏板 (None表示空，Some表示有方块)
    pub board: [[Option<TetrominoType>; BOARD_WIDTH]; BOARD_HEIGHT],
    /// 当前方块
    pub current_piece: Option<Tetromino>,
    /// 下一个方块
    pub next_piece: TetrominoType,
    /// 分数
    pub score: u32,
    /// 消除行数
    pub lines: u32,
    /// 游戏状态
    pub state: GameState,
    /// 下落计数器
    pub drop_counter: u32,
    /// 下落速度（毫秒）
    pub drop_speed: u32,
}

impl TetrisGame {
    pub fn new() -> Self {
        let first_piece = TetrominoType::random();
        let next_piece = TetrominoType::random();
        
        Self {
            board: [[None; BOARD_WIDTH]; BOARD_HEIGHT],
            current_piece: Some(Tetromino::new(first_piece)),
            next_piece,
            score: 0,
            lines: 0,
            state: GameState::Playing,
            drop_counter: 0,
            drop_speed: 500, // 初始500ms下落一格
        }
    }

    /// 重置游戏
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// 检查位置是否有效
    pub fn is_valid_position(&self, piece: &Tetromino) -> bool {
        let shape = piece.shape();
        for y in 0..4 {
            for x in 0..4 {
                if shape[y][x] {
                    let board_x = piece.x + x as i16;
                    let board_y = piece.y + y as i16;

                    // 检查边界
                    if board_x < 0 || board_x >= BOARD_WIDTH as i16 {
                        return false;
                    }
                    if board_y < 0 || board_y >= BOARD_HEIGHT as i16 {
                        return false;
                    }

                    // 检查碰撞
                    if board_y >= 0 && self.board[board_y as usize][board_x as usize].is_some() {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// 锁定当前方块到游戏板
    pub fn lock_piece(&mut self) {
        if let Some(ref piece) = self.current_piece {
            let shape = piece.shape();
            for y in 0..4 {
                for x in 0..4 {
                    if shape[y][x] {
                        let board_x = piece.x + x as i16;
                        let board_y = piece.y + y as i16;
                        if board_y >= 0 && board_y < BOARD_HEIGHT as i16 
                           && board_x >= 0 && board_x < BOARD_WIDTH as i16 {
                            self.board[board_y as usize][board_x as usize] = Some(piece.kind);
                        }
                    }
                }
            }
        }

        // 消除完整的行
        self.clear_lines();

        // 生成新方块
        self.spawn_new_piece();
    }

    /// 消除完整的行
    pub fn clear_lines(&mut self) {
        let mut lines_cleared = 0;

        for y in (0..BOARD_HEIGHT).rev() {
            // 检查这一行是否完整
            let is_full = self.board[y].iter().all(|cell| cell.is_some());
            
            if is_full {
                lines_cleared += 1;
                // 将上面的行下移
                for yy in (0..y).rev() {
                    self.board[yy + 1] = self.board[yy];
                }
                // 清空最上面一行
                self.board[0] = [None; BOARD_WIDTH];
            }
        }

        if lines_cleared > 0 {
            self.lines += lines_cleared as u32;
            // 计分：1行100分，2行300分，3行500分，4行800分
            self.score += match lines_cleared {
                1 => 100,
                2 => 300,
                3 => 500,
                4 => 800,
                _ => 0,
            };

            // 每消除10行加速
            if self.lines / 10 > (self.lines - lines_cleared as u32) / 10 {
                self.drop_speed = self.drop_speed.saturating_sub(50);
                if self.drop_speed < 100 {
                    self.drop_speed = 100;
                }
            }
        }
    }

    /// 生成新方块
    pub fn spawn_new_piece(&mut self) {
        let new_piece = Tetromino::new(self.next_piece);
        self.next_piece = TetrominoType::random();

        if self.is_valid_position(&new_piece) {
            self.current_piece = Some(new_piece);
        } else {
            // 游戏结束
            self.state = GameState::GameOver;
            self.current_piece = None;
        }
    }

    /// 尝试旋转当前方块
    pub fn try_rotate(&mut self) {
        if let Some(ref piece) = self.current_piece {
            let mut new_piece = piece.clone();
            new_piece.rotate();
            
            // 如果旋转后位置无效，尝试墙踢
            if !self.is_valid_position(&new_piece) {
                // 尝试左移
                new_piece.x -= 1;
                if self.is_valid_position(&new_piece) {
                    self.current_piece = Some(new_piece);
                    return;
                }
                new_piece.x += 1;

                // 尝试右移
                new_piece.x += 1;
                if self.is_valid_position(&new_piece) {
                    self.current_piece = Some(new_piece);
                    return;
                }
                new_piece.x -= 1;

                // 恢复原状态 - 不旋转
                return;
            }
            self.current_piece = Some(new_piece);
        }
    }

    /// 尝试左移
    pub fn try_move_left(&mut self) {
        if let Some(ref piece) = self.current_piece {
            let mut new_piece = piece.clone();
            new_piece.move_left();
            if self.is_valid_position(&new_piece) {
                self.current_piece = Some(new_piece);
            }
        }
    }

    /// 尝试右移
    pub fn try_move_right(&mut self) {
        if let Some(ref piece) = self.current_piece {
            let mut new_piece = piece.clone();
            new_piece.move_right();
            if self.is_valid_position(&new_piece) {
                self.current_piece = Some(new_piece);
            }
        }
    }

    /// 尝试下移
    pub fn try_move_down(&mut self) -> bool {
        if let Some(ref piece) = self.current_piece {
            let mut new_piece = piece.clone();
            new_piece.move_down();
            if !self.is_valid_position(&new_piece) {
                self.lock_piece();
                return false;
            }
            self.current_piece = Some(new_piece);
            return true;
        }
        false
    }

    /// 硬降（直接落到底部）
    pub fn hard_drop(&mut self) {
        while self.try_move_down() {}
    }

    /// 更新游戏状态
    pub fn update(&mut self, delta_ms: u32) {
        if self.state != GameState::Playing {
            return;
        }

        self.drop_counter += delta_ms;
        if self.drop_counter >= self.drop_speed {
            self.drop_counter = 0;
            self.try_move_down();
        }
    }
}

impl Default for TetrisGame {
    fn default() -> Self {
        Self::new()
    }
}
