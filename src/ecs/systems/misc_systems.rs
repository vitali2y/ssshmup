#![allow(clippy::type_complexity)]
use crate::ecs::components::*;
use crate::ecs::resources::*;
use specs::prelude::*;

use ggez::graphics::Rect;

pub struct IntegrateSys;
impl<'a> System<'a> for IntegrateSys {
    type SystemData = (WriteStorage<'a, Position>, ReadStorage<'a, Velocity>);

    fn run(&mut self, (mut positions, vels): Self::SystemData) {
        (&mut positions, &vels).par_join().for_each(|(pos, vel)| {
            pos.0 += vel.0;
        });
    }
}

pub struct IFrameSys;
impl<'a> System<'a> for IFrameSys {
    type SystemData = WriteStorage<'a, HP>;

    fn run(&mut self, mut hp_storage: Self::SystemData) {
        (&mut hp_storage).par_join().for_each(|hp| {
            if hp.iframes > 0 {
                hp.iframes -= 1;
            }
        });
    }
}

pub struct BulletCollSys;
impl<'a> System<'a> for BulletCollSys {
    type SystemData = (
        ReadStorage<'a, Bullet>,
        ReadStorage<'a, Hitbox>,
        WriteStorage<'a, HP>,
        WriteStorage<'a, Position>,
        WriteStorage<'a, AnimatedSprite>,
        Entities<'a>,
        Read<'a, AnimatedSprites>,
        Read<'a, PlayerEntity>,
        Write<'a, HPText>,
        Read<'a, Sounds>,
        Write<'a, QueuedSounds>,
    );

    fn run(
        &mut self,
        (
            bullets,
            hitboxes,
            mut hp_storage,
            mut positions,
            mut animated_sprite_storage,
            entities,
            animated_sprites,
            player_entity,
            mut hp_text,
            sounds,
            mut queued_sounds,
        ): Self::SystemData,
    ) {
        let mut explosion_positions: Vec<Point> = Vec::new();

        (&bullets, &positions, &hitboxes, &entities)
            .join()
            .for_each(|(bullet, pos, bullet_hitbox, bullet_entity)| {
                let bullet_rect = Rect::new(
                    pos.0.x + bullet_hitbox.0.x,
                    pos.0.y + bullet_hitbox.0.y,
                    bullet_hitbox.1,
                    bullet_hitbox.2,
                );
                if !(-10.0..crate::SCREEN_WIDTH + 10.0).contains(&pos.0.x)
                    || !(-10.0..crate::SCREEN_HEIGHT).contains(&pos.0.y)
                {
                    if entities.delete(bullet_entity).is_err() {
                        log::warn!("error deleting offscreen bullet entity")
                    }
                } else {
                    (&mut hp_storage, &positions, &hitboxes, &entities)
                        .join()
                        .for_each(|(hp, collided_pos, hitbox, entity)| {
                            if (bullet.damages_player() && entity == player_entity.0)
                                || (bullet.damages_enemy() && entity != player_entity.0)
                                    && hp.remaining > 0
                            {
                                let collidee_rect = Rect::new(
                                    collided_pos.0.x + hitbox.0.x,
                                    collided_pos.0.y + hitbox.0.y,
                                    hitbox.1,
                                    hitbox.2,
                                );
                                if bullet_rect.overlaps(&collidee_rect) {
                                    if hp.remaining >= bullet.damage {
                                        hp.remaining -= bullet.damage;
                                    } else {
                                        hp.remaining = 0;
                                    }
                                    if entity == player_entity.0 {
                                        hp_text.needs_redraw = true;
                                    }
                                    explosion_positions.push(pos.0 + Vector::new(-20.0, -20.0));
                                    if entities.delete(bullet_entity).is_err() {
                                        log::warn!("error deleting collided bullet entity")
                                    }
                                }
                            }
                        });
                }
            });

        explosion_positions.iter().for_each(|pos| {
            entities
                .build_entity()
                .with(Position(*pos), &mut positions)
                .with(
                    animated_sprites
                        .0
                        .get("explosion")
                        .expect("error getting explosion sprite")
                        .clone(),
                    &mut animated_sprite_storage,
                )
                .build();
        });

        if !explosion_positions.is_empty() {
            if let Some(sound) = sounds.0.get("boom") {
                queued_sounds.0.push(sound.clone());
            } else {
                log::warn!("error playing explosion sound");
            }
        }
    }
}

pub struct AnimationSys;
impl<'a> System<'a> for AnimationSys {
    type SystemData = (WriteStorage<'a, AnimatedSprite>, Entities<'a>);

    fn run(&mut self, (mut animated_sprite_storage, entities): Self::SystemData) {
        (&mut animated_sprite_storage, &entities)
            .join()
            .for_each(|(animated_sprite, entity)| {
                animated_sprite.current_frame += 1;
                if animated_sprite.current_frame == animated_sprite.num_frames {
                    if animated_sprite.temporary {
                        if entities.delete(entity).is_err() {
                            log::warn!("error deleting finished animation entity");
                        }
                    } else {
                        animated_sprite.current_frame = 0;
                    }
                }
            })
    }
}

pub struct BulletTrackingSys;
impl<'a> System<'a> for BulletTrackingSys {
    type SystemData = (
        WriteStorage<'a, Velocity>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, Bullet>,
        Read<'a, PlayerEntity>,
    );

    fn run(&mut self, (mut vels, positions, bullets, player_entity): Self::SystemData) {
        if let Some(player_pos) = positions.get(player_entity.0) {
            let player_pos = player_pos.0;
            (&mut vels, &positions, &bullets)
                .par_join()
                .filter(|(_, _, bullet)| bullet.ty == BulletType::TrackingBullet)
                .for_each(|(vel, pos, _)| {
                    let direction = (player_pos - pos.0).normalize();
                    let target_vel = direction * 8.0;
                    vel.0 += (target_vel - vel.0) * 0.02;
                });
        }
    }
}

pub struct HPKillSys;
impl<'a> System<'a> for HPKillSys {
    type SystemData = (
        ReadStorage<'a, HP>,
        Entities<'a>,
        Read<'a, PlayerEntity>,
        Write<'a, Dead>,
        Read<'a, Sounds>,
        Write<'a, QueuedSounds>,
    );

    fn run(
        &mut self,
        (hp_storage, entities, player_entity, mut dead, sounds, mut queued_sounds): Self::SystemData,
    ) {
        (&hp_storage, &entities).join().for_each(|(hp, entity)| {
            if hp.remaining == 0 {
                entities.delete(entity).expect("error deleting dead entity");
                if entity == player_entity.0 {
                    dead.0 = true;
                    if let Some(sound) = sounds.0.get("dead") {
                        queued_sounds.0.push(sound.clone());
                    } else {
                        log::warn!("error getting death sound");
                    }
                }
            }
        });
    }
}

pub struct BounceBulletSys;
impl<'a> System<'a> for BounceBulletSys {
    type SystemData = (
        WriteStorage<'a, Bullet>,
        ReadStorage<'a, Position>,
        WriteStorage<'a, Velocity>,
        Entities<'a>,
    );

    fn run(&mut self, (mut bullets, positions, mut vels, entities): Self::SystemData) {
        (&mut bullets, &positions, &mut vels, &entities)
            .join()
            .for_each(|(bullet, pos, vel, entity)| {
                // can't mutate enum tuples in if let statement?
                // something weird happens but workaround is ok
                let mut new_bounce_ty: Option<BulletType> = None;
                if let BulletType::BouncingBullet(num_bounces) = bullet.ty {
                    if pos.0.x > crate::SCREEN_WIDTH && vel.0.x > 0.0
                        || pos.0.x < 0.0 && vel.0.x < 0.0
                    {
                        vel.0.x *= -1.0;
                        new_bounce_ty = Some(BulletType::BouncingBullet(
                            num_bounces - (1).min(num_bounces),
                        ));
                        if num_bounces == 0 {
                            entities
                                .delete(entity)
                                .expect("error deleting overbounced bullet");
                        }
                    }
                }
                if let Some(ty) = new_bounce_ty {
                    bullet.ty = ty;
                }
            });
    }
}
