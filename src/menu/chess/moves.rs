// Lógica de movimentos do xadrez: movimentos pseudo-legais, deslizantes e validação de xeque
use super::ChessGame;
use super::piece::{Piece, PieceColor, PieceType};

impl ChessGame {
    pub fn calculate_valid_moves(&mut self, row: usize, col: usize) {
        self.valid_moves.clear();

        let piece = match self.board[row][col] {
            Some(p) => p,
            None => return, // Nenhuma peça na casa selecionada
        };

        // 1. Obtém todos os movimentos considerando apenas as direções e obstáculos ("paredes")
        let pseudo_moves = self.get_pseudo_legal_moves(row, col, piece);

        // 2. Filtra os movimentos que deixariam ou manteriam o próprio Rei em xeque
        let mut legal_moves = Vec::new();

        for (tr, tc) in pseudo_moves {
            // Guarda o estado original da casa de destino para restaurar depois
            let original_destination = self.board[tr][tc];

            // Simula o movimento
            self.board[tr][tc] = Some(piece);
            self.board[row][col] = None;

            // Se o próprio rei NÃO está em xeque após o movimento, ele é válido!
            if !self.is_king_in_check(piece.color) {
                legal_moves.push((tr, tc));
            }

            // Desfaz a simulação (Restaura o tabuleiro)
            self.board[row][col] = Some(piece);
            self.board[tr][tc] = original_destination;
        }

        self.valid_moves = legal_moves;
    }

    // Calcula movimentos baseados puramente na mecânica da peça e colisões
    pub(super) fn get_pseudo_legal_moves(
        &self,
        row: usize,
        col: usize,
        piece: Piece,
    ) -> Vec<(usize, usize)> {
        let mut moves = Vec::new();
        let r = row as isize;
        let c = col as isize;

        match piece.piece_type {
            PieceType::Pawn => {
                // Define a direção: Branco sobe (row - 1), Preto desce (row + 1)
                let (dir, start_row) = match piece.color {
                    PieceColor::White => (-1, 6),
                    PieceColor::Black => (1, 1),
                };

                // 1 passo à frente (casa precisa estar vazia)
                let next_r = r + dir;
                if next_r >= 0 && next_r < 8 && self.board[next_r as usize][col].is_none() {
                    moves.push((next_r as usize, col));

                    // 2 passos à frente (se estiver na linha inicial e caminho livre)
                    if row == start_row {
                        let next_r2 = r + (dir * 2);
                        if self.board[next_r2 as usize][col].is_none() {
                            moves.push((next_r2 as usize, col));
                        }
                    }
                }

                // Capturas diagonais (precisa ter peça inimiga)
                for &dc in &[-1, 1] {
                    let next_c = c + dc;
                    if next_r >= 0 && next_r < 8 && next_c >= 0 && next_c < 8 {
                        if let Some(target) = self.board[next_r as usize][next_c as usize] {
                            if target.color != piece.color {
                                moves.push((next_r as usize, next_c as usize));
                            }
                        }
                    }
                }
            }

            PieceType::Knight => {
                let knight_offsets = [
                    (-2, -1),
                    (-2, 1),
                    (-1, -2),
                    (-1, 2),
                    (1, -2),
                    (1, 2),
                    (2, -1),
                    (2, 1),
                ];
                for &(dr, dc) in &knight_offsets {
                    let tr = r + dr;
                    let tc = c + dc;
                    if tr >= 0 && tr < 8 && tc >= 0 && tc < 8 {
                        if let Some(target) = self.board[tr as usize][tc as usize] {
                            if target.color != piece.color {
                                moves.push((tr as usize, tc as usize));
                            }
                        } else {
                            moves.push((tr as usize, tc as usize));
                        }
                    }
                }
            }

            PieceType::King => {
                let king_offsets = [
                    (-1, -1),
                    (-1, 0),
                    (-1, 1),
                    (0, -1),
                    (0, 1),
                    (1, -1),
                    (1, 0),
                    (1, 1),
                ];
                for &(dr, dc) in &king_offsets {
                    let tr = r + dr;
                    let tc = c + dc;
                    if tr >= 0 && tr < 8 && tc >= 0 && tc < 8 {
                        if let Some(target) = self.board[tr as usize][tc as usize] {
                            if target.color != piece.color {
                                moves.push((tr as usize, tc as usize));
                            }
                        } else {
                            moves.push((tr as usize, tc as usize));
                        }
                    }
                }
            }

            // Peças Deslizantes (Rook, Bishop, Queen) controladas por loops de colisão ("parede")
            PieceType::Rook => {
                let directions = [(0, 1), (0, -1), (1, 0), (-1, 0)];
                self.add_sliding_moves(r, c, &directions, piece.color, &mut moves);
            }
            PieceType::Bishop => {
                let directions = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
                self.add_sliding_moves(r, c, &directions, piece.color, &mut moves);
            }
            PieceType::Queen => {
                let directions = [
                    (0, 1),
                    (0, -1),
                    (1, 0),
                    (-1, 0),
                    (1, 1),
                    (1, -1),
                    (-1, 1),
                    (-1, -1),
                ];
                self.add_sliding_moves(r, c, &directions, piece.color, &mut moves);
            }
        }

        moves
    }

    // Auxiliar para as peças que "deslizam" até encontrar uma parede ou peça
    pub(super) fn add_sliding_moves(
        &self,
        start_r: isize,
        start_c: isize,
        directions: &[(isize, isize)],
        color: PieceColor,
        moves: &mut Vec<(usize, usize)>,
    ) {
        for &(dr, dc) in directions {
            let mut r = start_r + dr;
            let mut c = start_c + dc;

            while r >= 0 && r < 8 && c >= 0 && c < 8 {
                let tr = r as usize;
                let tc = c as usize;

                if let Some(target) = self.board[tr][tc] {
                    if target.color != color {
                        moves.push((tr, tc)); // Captura a peça inimiga...
                    }
                    break; // ...mas a linha é bloqueada ("parede"), encerra esta direção.
                }

                moves.push((tr, tc)); // Casa vazia, continua deslizando
                r += dr;
                c += dc;
            }
        }
    }

    // Verifica se o Rei de uma determinada cor está sob ataque direto
    pub fn is_king_in_check(&self, color: PieceColor) -> bool {
        // 1. Localiza o Rei no tabuleiro
        let mut king_pos = None;
        for r in 0..8 {
            for c in 0..8 {
                if let Some(p) = self.board[r][c] {
                    if p.piece_type == PieceType::King && p.color == color {
                        king_pos = Some((r, c));
                        break;
                    }
                }
            }
        }

        let (kr, kc) = match king_pos {
            Some(pos) => (pos.0 as isize, pos.1 as isize),
            None => return false, // Salvaguarda caso o rei não exista (ex: testes)
        };

        // 2. Traça raios a partir do Rei para ver se alguma peça inimiga o ameaça

        // Linhas retas (Torres e Rainhas inimigas)
        let straight = [(0, 1), (0, -1), (1, 0), (-1, 0)];
        for &(dr, dc) in &straight {
            let mut r = kr + dr;
            let mut c = kc + dc;
            while r >= 0 && r < 8 && c >= 0 && c < 8 {
                if let Some(p) = self.board[r as usize][c as usize] {
                    if p.color != color
                        && (p.piece_type == PieceType::Rook || p.piece_type == PieceType::Queen)
                    {
                        return true;
                    }
                    break; // Linha interrompida por outra peça
                }
                r += dr;
                c += dc;
            }
        }

        // Diagonais (Bispos e Rainhas inimigas)
        let diagonals = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
        for &(dr, dc) in &diagonals {
            let mut r = kr + dr;
            let mut c = kc + dc;
            while r >= 0 && r < 8 && c >= 0 && c < 8 {
                if let Some(p) = self.board[r as usize][c as usize] {
                    if p.color != color
                        && (p.piece_type == PieceType::Bishop || p.piece_type == PieceType::Queen)
                    {
                        return true;
                    }
                    break;
                }
                r += dr;
                c += dc;
            }
        }

        // Cavalos Inimigos
        let knight_offsets = [
            (-2, -1),
            (-2, 1),
            (-1, -2),
            (-1, 2),
            (1, -2),
            (1, 2),
            (2, -1),
            (2, 1),
        ];
        for &(dr, dc) in &knight_offsets {
            let r = kr + dr;
            let c = kc + dc;
            if r >= 0 && r < 8 && c >= 0 && c < 8 {
                if let Some(p) = self.board[r as usize][c as usize] {
                    if p.color != color && p.piece_type == PieceType::Knight {
                        return true;
                    }
                }
            }
        }

        // Peões Inimigos (Olha na diagonal de onde os peões inimigos atacariam)
        let pawn_attack_dir = match color {
            PieceColor::White => -1, // Peões pretos vêm de cima atacando para baixo (-1 na visão do Rei Branco)
            PieceColor::Black => 1, // Peões brancos vêm de baixo atacando para cima (+1 na visão do Rei Preto)
        };
        let pr = kr + pawn_attack_dir;
        for &dc in &[-1, 1] {
            let pc = kc + dc;
            if pr >= 0 && pr < 8 && pc >= 0 && pc < 8 {
                if let Some(p) = self.board[pr as usize][pc as usize] {
                    if p.color != color && p.piece_type == PieceType::Pawn {
                        return true;
                    }
                }
            }
        }

        // Rei Inimigo (Evita que reis fiquem colados)
        let king_offsets = [
            (-1, -1),
            (-1, 0),
            (-1, 1),
            (0, -1),
            (0, 1),
            (1, -1),
            (1, 0),
            (1, 1),
        ];
        for &(dr, dc) in &king_offsets {
            let r = kr + dr;
            let c = kc + dc;
            if r >= 0 && r < 8 && c >= 0 && c < 8 {
                if let Some(p) = self.board[r as usize][c as usize] {
                    if p.color != color && p.piece_type == PieceType::King {
                        return true;
                    }
                }
            }
        }

        false
    }
}
