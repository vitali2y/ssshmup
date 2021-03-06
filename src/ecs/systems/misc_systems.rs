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
        WriteStorage<'a, Bullet>,
        ReadStorage<'a, Hitbox>,
        WriteStorage<'a, HP>,
        ReadStorage<'a, Position>,
        WriteStorage<'a, Player>,
        WriteStorage<'a, Velocity>,
        ReadStorage<'a, Deflected>,
        Entities<'a>,
        Read<'a, AnimatedSprites>,
        Read<'a, PlayerEntity>,
        Write<'a, HPText>,
        Read<'a, Sounds>,
        Write<'a, QueuedSounds>,
        Read<'a, LazyUpdate>,
    );

    fn run(
        &mut self,
        (
            mut bullets,
            hitboxes,
            mut hp_storage,
            positions,
            mut players,
            mut vels,
            deflecteds,
            entities,
            animated_sprites,
            player_entity,
            mut hp_text,
            sounds,
            mut queued_sounds,
            lazy_update,
        ): Self::SystemData,
    ) {
        let mut atleast_one_explosion = false;
        let mut atleast_one_deflection = false;
        let mut player_deflection_hp = 0;

        let sprite = animated_sprites
            .0
            .get("explosion")
            .expect("error getting explosion sprite");
        (&mut bullets, &positions, &hitboxes, &entities)
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
                                    if entity == player_entity.0 {
                                        hp_text.needs_redraw = true;
                                        let player = players
                                            .get_mut(player_entity.0)
                                            .expect("error getting player entity");

                                        let player_vel = vels
                                            .get(player_entity.0)
                                            .expect("error getting player vel")
                                            .0;

                                        if player.deflector_timer > 0 {
                                            let bullet_vel = &mut vels
                                                .get_mut(bullet_entity)
                                                .expect("error getting deflected bullet vel")
                                                .0;

                                            let normal = {
                                                let bullet_center = Point::new(
                                                    bullet_rect.x + bullet_rect.w / 2.0,
                                                    bullet_rect.y + bullet_rect.h / 2.0,
                                                );
                                                let player_center = Point::new(
                                                    collidee_rect.x + collidee_rect.w / 2.0,
                                                    collidee_rect.y + collidee_rect.h / 2.0,
                                                );

                                                (bullet_center - player_center).normalize()
                                            };

                                            let bullet_vel_dot_normal =
                                                bullet_vel.x * normal.x + bullet_vel.y * normal.y;

                                            // dbg!(bullet_vel, 2.0 * bullet_vel_dot_normal * normal);
                                            *bullet_vel -= 2.0 * bullet_vel_dot_normal * normal;
                                            *bullet_vel += player_vel;
                                            if bullet_vel.x.abs() < 8.0 {
                                                bullet_vel.x = 0.0;
                                            }

                                            let bullet_speed = bullet_vel.norm();
                                            if bullet_speed < 8.0 {
                                                *bullet_vel *= 8.0 / bullet_speed;
                                            }

                                            if let BulletType::TrackingBullet(frames_remaining) =
                                                bullet.ty
                                            {
                                                if frames_remaining > 10 {
                                                    bullet.ty = BulletType::TrackingBullet(10);
                                                }
                                            } else {
                                                bullet.damages_who = DamagesWho::Enemy;
                                                bullet.damage *= 3;
                                            }

                                            lazy_update.insert(bullet_entity, Deflected::default());

                                            player.deflector_timer = player.deflector_frames * 2;
                                            atleast_one_deflection = true;
                                            return;
                                        }
                                    }

                                    if entities.delete(bullet_entity).is_err() {
                                        log::warn!("error deleting collided bullet entity")
                                    }
                                    let explosion = entities.create();
                                    lazy_update.insert(explosion, *pos);
                                    lazy_update.insert(explosion, sprite.clone());
                                    atleast_one_explosion = true;

                                    if deflecteds.get(bullet_entity).is_some() {
                                        player_deflection_hp += bullet.damage / 3 * 2;
                                        // deflected bullets do triple damage, and we want them to heal double damage
                                    }
                                    hp.remaining -= bullet.damage.min(hp.remaining);
                                }
                            }
                        });
                }
            });

        if atleast_one_explosion {
            if let Some(sound) = sounds.0.get("boom") {
                queued_sounds.0.push(sound.clone());
            } else {
                log::warn!("error playing explosion sound");
            }
        }
        if atleast_one_deflection {
            if let Some(sound) = sounds.0.get("deflect") {
                queued_sounds.0.push(sound.clone());
            } else {
                log::warn!("error playing deflection sound");
            }
        }

        if player_deflection_hp > 0 {
            hp_storage.get_mut(player_entity.0).unwrap().remaining += player_deflection_hp;
            hp_text.needs_redraw = true;
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
        WriteStorage<'a, Bullet>,
        Read<'a, PlayerEntity>,
        Read<'a, AnimatedSprites>,
        Read<'a, Sounds>,
        Write<'a, QueuedSounds>,
        Entities<'a>,
        Read<'a, LazyUpdate>,
    );

    fn run(
        &mut self,
        (
            mut vels,
            positions,
            mut bullets,
            player_entity,
            animated_sprites,
            sounds,
            mut queued_sounds,
            entities,
            lazy_update,
        ): Self::SystemData,
    ) {
        if let Some(player_pos) = positions.get(player_entity.0) {
            let player_pos = player_pos.0;
            let mut atleast_one_explosion = std::sync::atomic::AtomicBool::new(false);
            let explosion_sprite = animated_sprites
                .0
                .get("explosion")
                .expect("error getting explosion sprite");

            (&mut vels, &positions, &mut bullets, &entities)
                .join()
                .for_each(|(vel, pos, bullet, entity)| {
                    let mut new_ty: Option<BulletType> = None;
                    if let BulletType::TrackingBullet(frames_remaining) = bullet.ty {
                        if frames_remaining == 0 {
                            let explosion_entity = entities.create();
                            lazy_update.insert(explosion_entity, *pos);
                            lazy_update.insert(explosion_entity, explosion_sprite.clone());
                            entities
                                .delete(entity)
                                .expect("error deleting dead tracking bullet");
                            *atleast_one_explosion.get_mut() = true;
                            return;
                        }
                        new_ty = Some(BulletType::TrackingBullet(frames_remaining - 1));
                        let direction = (player_pos - pos.0).normalize();
                        let target_vel = direction * 7.0;
                        vel.0 += (target_vel - vel.0) * 0.02;
                    }
                    if let Some(ty) = new_ty {
                        bullet.ty = ty;
                    }
                });

            if atleast_one_explosion.into_inner() {
                if let Some(explosion_sound) = sounds.0.get("boom") {
                    queued_sounds.0.push(explosion_sound.clone());
                } else {
                    log::warn!("error getting explosion sound");
                }
            }
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
