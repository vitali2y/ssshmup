use ggez::graphics::{Color, Image, spritebatch::SpriteBatch};
use ggez::nalgebra::{Point2, Vector2};

use specs::prelude::*;
use specs::Component;

use std::collections::HashMap;

pub type Point = Point2<f32>;
pub type Vector = Vector2<f32>;

#[derive(Clone, Copy, Debug, PartialEq, Component)]
#[storage(VecStorage)]
pub struct Position(pub Point);

impl Into<Point> for Position {
    fn into(self) -> Point {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Component)]
#[storage(VecStorage)]
pub struct Velocity(pub Vector);
impl Default for Velocity {
    fn default() -> Self {
        Velocity(Vector::new(0.0, 0.0))
    }
}

#[derive(Clone, Debug, PartialEq, Component)]
#[storage(VecStorage)]
pub struct Sprite(pub Image);

#[derive(Clone, Copy, Debug, PartialEq, Component)]
#[storage(VecStorage)]
pub struct ColorRect {
    pub color: Color,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Component)]
#[storage(VecStorage)]
pub struct HP(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BulletType {
    BasicBullet,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Component)]
#[storage(VecStorage)]
pub struct Bullet {
    pub damage: u32,
    pub ty: BulletType,
}

pub type BulletTuple = (Position, Velocity, Bullet);
pub fn new_bullet(ty: BulletType, pos: Point, start_vel: Vector) -> BulletTuple {
    let (damage, speed, w, h, color) = match ty {
        BulletType::BasicBullet => (1, -8.0, 5.0, 10.0, Color::new(0.0, 1.0, 1.0, 1.0)),
    };

    let bullet = Bullet { damage, ty };

    let pos: Point = [pos.x, pos.y - 16.0].into();
    (
        Position(pos),
        Velocity([0.0, speed + start_vel.y.min(0.0)].into()),
        bullet,
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Component)]
#[storage(VecStorage)]
pub enum Enemy {
    BasicEnemy,
}

pub type EnemyTuple = (Position, Velocity, Enemy, HP);
pub fn new_enemy(enemy_type: Enemy, pos: Point) -> EnemyTuple {
    let pos = Position(pos);
    let vel = Velocity::default();
    let hp = match enemy_type {
        Enemy::BasicEnemy => 1,
    };

    (pos, vel, enemy_type, HP(hp))
}

pub fn create_enemy(world: &mut World, enemy: &EnemyTuple) -> Entity {
    let sprite = {
        let sprites = &world.fetch::<Sprites>().0;
        sprites
            .get(match enemy.2 {
                Enemy::BasicEnemy => "enemy1",
            })
            .unwrap()
            .clone()
    };

    world
        .create_entity()
        .with(enemy.0)
        .with(enemy.1)
        .with(enemy.2)
        .with(enemy.3)
        .with(Sprite(sprite))
        .build()
}

#[derive(Clone, Copy, Debug, PartialEq, Component)]
#[storage(HashMapStorage)]
pub struct Player {
    pub bullet_type: BulletType,
    pub reload_speed: u32,
    pub reload_timer: u32,
}

impl Default for Player {
    fn default() -> Self {
        Player {
            bullet_type: BulletType::BasicBullet,
            reload_speed: 10,
            reload_timer: 10,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerEntity(pub Entity);

impl Default for PlayerEntity {
    fn default() -> Self {
        panic!("something has gone terribly wrong")
    }
}

pub type PlayerTuple = (Position, Velocity, HP, Sprite, Player);
pub fn new_player(sprite: Image, hp: u32) -> PlayerTuple {
    let pos = Position(
        [
            crate::SCREEN_WIDTH / 2.0 - 25.0,
            crate::SCREEN_HEIGHT * 0.75,
        ]
        .into(),
    );
    let vel = Velocity::default();
    let hp = HP(hp);
    // let rect = ColorRect {
    //     color: Color::new(1.0, 1.0, 1.0, 1.0),
    //     w: 50.0,
    //     h: 80.0,
    // };

    (pos, vel, hp, Sprite(sprite), Player::default())
}

pub fn create_player(world: &mut World, player: PlayerTuple) -> Entity {
    world
        .create_entity()
        .with(player.0)
        .with(player.1)
        .with(player.2)
        .with(player.3)
        .with(player.4)
        .build()
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct StarInfo {
    pub num_stars: usize,
    pub size: f32,
    pub size_variance: f32,
    pub vel: f32,
    pub vel_variance: f32,
}

impl StarInfo {
    pub fn new_star(&self) -> (Position, Velocity, ColorRect) {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let x = rng.gen_range(0.0, crate::SCREEN_WIDTH);
        let y = rng.gen_range(-crate::SCREEN_WIDTH, 0.0);
        let y_vel = rng.gen_range(self.vel - self.vel_variance, self.vel + self.vel_variance);
        let size = rng.gen_range(
            self.size - self.size_variance,
            self.size + self.size_variance,
        );

        let pos = [x, y].into();
        let vel = [0.0, y_vel].into();
        let color_rect = ColorRect {
            color: ggez::graphics::WHITE,
            w: size,
            h: size,
        };

        (Position(pos), Velocity(vel), color_rect)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Component, Default)]
#[storage(NullStorage)]
pub struct Star;

#[derive(Clone, Default)]
pub struct Sprites(pub HashMap<String, Image>);

#[derive(Clone, Default)]
pub struct BulletSpriteBatch(pub SpriteBatch);
