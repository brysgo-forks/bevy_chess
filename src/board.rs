use crate::pieces::*;
use bevy::{app::AppExit, prelude::*};
use bevy_mod_picking::*;
use chess::{
    ChessMove, Color as PieceColor, File, Game as ChessGame, Piece as PieceType, Rank, Square,
};

pub struct Game {
    pub chess_game: ChessGame,
}
impl Default for Game {
    fn default() -> Self {
        Game {
            chess_game: ChessGame::new(),
        }
    }
}

fn create_board(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: Res<SquareMaterials>,
) {
    // Add meshes
    let mesh = meshes.add(Mesh::from(shape::Plane { size: 1. }));

    // Spawn 64 squares
    for i in 0..8 {
        for j in 0..8 {
            commands
                .spawn_bundle(PbrBundle {
                    mesh: mesh.clone(),
                    // Change material according to position to get alternating pattern
                    material: if (i + j + 1) % 2 == 0 {
                        materials.white_color.clone()
                    } else {
                        materials.black_color.clone()
                    },
                    transform: Transform::from_translation(Vec3::new(i as f32, 0., j as f32)),
                    ..Default::default()
                })
                .insert_bundle(PickableBundle::default())
                .insert(Square::make_square(
                    Rank::from_index(i),
                    File::from_index(j),
                ));
        }
    }
}

fn color_squares(
    selected_square: Res<SelectedSquare>,
    materials: Res<SquareMaterials>,
    mut query: Query<(Entity, &Square, &mut Handle<StandardMaterial>)>,
    picking_camera_query: Query<&PickingCamera>,
) {
    // Get entity under the cursor, if there is one
    let top_entity = match picking_camera_query.iter().last() {
        Some(picking_camera) => match picking_camera.intersect_top() {
            Some((entity, _intersection)) => Some(entity),
            None => None,
        },
        None => None,
    };

    for (entity, square, mut material) in query.iter_mut() {
        // Change the material
        *material = if Some(entity) == top_entity {
            materials.highlight_color.clone()
        } else if Some(entity) == selected_square.entity {
            materials.selected_color.clone()
        } else if (square.get_rank().to_index() + square.get_file().to_index() + 1) % 2 == 0 {
            materials.white_color.clone()
        } else {
            materials.black_color.clone()
        };
    }
}

struct SquareMaterials {
    highlight_color: Handle<StandardMaterial>,
    selected_color: Handle<StandardMaterial>,
    black_color: Handle<StandardMaterial>,
    white_color: Handle<StandardMaterial>,
}

impl FromWorld for SquareMaterials {
    fn from_world(world: &mut World) -> Self {
        let world = world.cell();
        let mut materials = world
            .get_resource_mut::<Assets<StandardMaterial>>()
            .unwrap();
        SquareMaterials {
            highlight_color: materials.add(Color::rgb(0.8, 0.3, 0.3).into()),
            selected_color: materials.add(Color::rgb(0.9, 0.1, 0.1).into()),
            black_color: materials.add(Color::rgb(0., 0.1, 0.1).into()),
            white_color: materials.add(Color::rgb(1., 0.9, 0.9).into()),
        }
    }
}

#[derive(Default)]
struct SelectedSquare {
    entity: Option<Entity>,
}
#[derive(Default)]
struct SelectedPiece {
    entity: Option<Entity>,
}

fn select_square(
    mouse_button_inputs: Res<Input<MouseButton>>,
    mut selected_square: ResMut<SelectedSquare>,
    mut selected_piece: ResMut<SelectedPiece>,
    squares_query: Query<&Square>,
    picking_camera_query: Query<&PickingCamera>,
) {
    // Only run if the left button is pressed
    if !mouse_button_inputs.just_pressed(MouseButton::Left) {
        return;
    }

    // Get the square under the cursor and set it as the selected
    if let Some(picking_camera) = picking_camera_query.iter().last() {
        if let Some((square_entity, _intersection)) = picking_camera.intersect_top() {
            if let Ok(_square) = squares_query.get(square_entity) {
                // Mark it as selected
                selected_square.entity = Some(square_entity);
            }
        } else {
            // Player clicked outside the board, deselect everything
            selected_square.entity = None;
            selected_piece.entity = None;
        }
    }
}

fn select_piece(
    selected_square: Res<SelectedSquare>,
    mut selected_piece: ResMut<SelectedPiece>,
    game: Res<Game>,
    squares_query: Query<&Square>,
    pieces_query: Query<(Entity, &Piece)>,
) {
    if !selected_square.is_changed() {
        return;
    }

    let square_entity = if let Some(entity) = selected_square.entity {
        entity
    } else {
        return;
    };

    let square = if let Ok(square) = squares_query.get(square_entity) {
        square
    } else {
        return;
    };

    if selected_piece.entity.is_none() {
        // Select the piece in the currently selected square
        for (piece_entity, piece) in pieces_query.iter() {
            if piece.square == *square
                && piece.color == game.chess_game.current_position().side_to_move()
            {
                // piece_entity is now the entity in the same square
                selected_piece.entity = Some(piece_entity);
                break;
            }
        }
    }
}

fn move_piece(
    mut commands: Commands,
    selected_square: Res<SelectedSquare>,
    selected_piece: Res<SelectedPiece>,
    mut game: ResMut<Game>,
    squares_query: Query<&Square>,
    mut pieces_query: Query<(Entity, &mut Piece)>,
    mut reset_selected_event: EventWriter<ResetSelectedEvent>,
) {
    let mut piece_index_opt: Option<usize> = None;
    let mut entity_pieces: Vec<(Entity, Mut<Piece>)> = pieces_query
        .iter_mut()
        .enumerate()
        .map(|(i, (entity, a_piece))| {
            if let Some(selected_piece_entity) = selected_piece.entity {
                if selected_piece_entity == entity {
                    piece_index_opt = Some(i);
                }
            }
            return (entity, a_piece);
        })
        .collect();
    let piece_index = if let Some(piece_index) = piece_index_opt {
        piece_index
    } else {
        return;
    };
    if !selected_square.is_changed() {
        return;
    }

    let square_entity = if let Some(entity) = selected_square.entity {
        entity
    } else {
        return;
    };

    let square = if let Ok(square) = squares_query.get(square_entity) {
        square
    } else {
        return;
    };

    // Move the selected piece to the selected square
    let old_square = entity_pieces[piece_index].1.square;
    let new_square = *square;
    let piece_color = entity_pieces[piece_index].1.color;
    let piece_type = entity_pieces[piece_index].1.piece_type;
    // Check if promotion
    let promotion: Option<PieceType> = match piece_type {
        PieceType::Pawn => match piece_color {
            PieceColor::Black => {
                if new_square.get_rank() == Rank::First {
                    Some(PieceType::Queen)
                } else {
                    None
                }
            }
            PieceColor::White => {
                if new_square.get_rank() == Rank::Eighth {
                    Some(PieceType::Queen)
                } else {
                    None
                }
            }
        },
        _ => None,
    };
    let m = ChessMove::new(old_square, new_square, promotion);
    let old_board = game.chess_game.current_position().to_owned();
    if game.chess_game.make_move(m) {
        for (entity, a_piece) in entity_pieces.iter_mut() {
            {
                // check if it is the piece we are moving
                if a_piece.square == old_square {
                    a_piece.square = new_square;
                    if let Some(promotion_piece) = promotion {
                        a_piece.piece_type = promotion_piece;
                        commands.entity(*entity).insert(Promoted);
                    }
                }
                // check if piece where we moved to
                else if a_piece.square == new_square {
                    let captured_piece = old_board.piece_on(new_square);
                    if captured_piece.is_some() {
                        // Mark the piece as taken
                        commands.entity(*entity).insert(Taken);
                    }
                }

                // check for castle move
                if a_piece.piece_type == PieceType::Rook && a_piece.color == piece_color {
                    let horizontal_movement = old_square.get_file().to_index() as i8
                        - new_square.get_file().to_index() as i8;
                    let castles = piece_type == PieceType::King && horizontal_movement.abs() > 1;

                    if castles {
                        if horizontal_movement > 0 {
                            // castle to left side of board (towards A rank)
                            if a_piece.square.get_file() == File::A {
                                match new_square.right() {
                                    Some(rook_square) => a_piece.square = rook_square,
                                    None => {}
                                }
                            }
                        } else {
                            // castle to right side of board (towards H rank)
                            if a_piece.square.get_file() == File::H {
                                match new_square.left() {
                                    Some(rook_square) => a_piece.square = rook_square,
                                    None => {}
                                }
                            }
                        }
                    }
                }

                // check for en passant
                if piece_type == PieceType::Pawn
                    && old_board.en_passant() == new_square.backward(piece_color)
                    && Some(a_piece.square) == old_board.en_passant()
                {
                    // Mark the piece as taken
                    commands.entity(*entity).insert(Taken);
                }
            }
        }
    }

    reset_selected_event.send(ResetSelectedEvent);
}

struct ResetSelectedEvent;

fn reset_selected(
    mut event_reader: EventReader<ResetSelectedEvent>,
    mut selected_square: ResMut<SelectedSquare>,
    mut selected_piece: ResMut<SelectedPiece>,
) {
    for _event in event_reader.iter() {
        selected_square.entity = None;
        selected_piece.entity = None;
    }
}

struct Taken;
fn despawn_taken_pieces(
    mut commands: Commands,
    mut app_exit_events: EventWriter<AppExit>,
    query: Query<(Entity, &Piece, &Taken)>,
) {
    for (entity, piece, _taken) in query.iter() {
        // If the king is taken, we should exit
        if piece.piece_type == PieceType::King {
            println!(
                "{} won! Thanks for playing!",
                match piece.color {
                    PieceColor::White => "Black",
                    PieceColor::Black => "White",
                }
            );
            app_exit_events.send(AppExit);
        }

        // Despawn piece and children
        commands.entity(entity).despawn_recursive();
    }
}

pub struct BoardPlugin;
impl Plugin for BoardPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<SelectedSquare>()
            .init_resource::<SelectedPiece>()
            .init_resource::<SquareMaterials>()
            .init_resource::<Game>()
            .add_event::<ResetSelectedEvent>()
            .add_startup_system(create_board.system())
            .add_system(color_squares.system())
            .add_system(select_square.system().label("select_square"))
            .add_system(
                // move_piece needs to run before select_piece
                move_piece
                    .system()
                    .after("select_square")
                    .before("select_piece"),
            )
            .add_system(
                select_piece
                    .system()
                    .after("select_square")
                    .label("select_piece"),
            )
            .add_system(despawn_taken_pieces.system())
            .add_system(reset_selected.system().after("select_square"));
    }
}
