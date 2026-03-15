use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Block {
    pub id: usize,
    pub command: String,
    pub start_row: usize, // absolute scrollback row
    pub end_row: Option<usize>,
    pub exit_code: Option<i32>,
    pub duration: Option<Duration>,
    pub started_at: Option<Instant>,
    pub working_dir: Option<String>,
}

#[derive(Debug, Default)]
pub struct BlockManager {
    blocks: Vec<Block>,
    next_id: usize,
    current_block: Option<usize>, // index into blocks
    prompt_row: Option<usize>,
    command_start_row: Option<usize>,
}

impl BlockManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Called on OSC 133;A (prompt start)
    pub fn on_prompt_start(&mut self, row: usize) {
        self.prompt_row = Some(row);
    }

    /// Called on OSC 133;B (command input start)
    pub fn on_command_start(&mut self, row: usize) {
        self.command_start_row = Some(row);
    }

    /// Called on OSC 133;C (command output start)
    pub fn on_output_start(&mut self, row: usize, command: String) {
        let id = self.next_id;
        self.next_id += 1;
        let block = Block {
            id,
            command,
            start_row: row,
            end_row: None,
            exit_code: None,
            duration: None,
            started_at: Some(Instant::now()),
            working_dir: None,
        };
        self.blocks.push(block);
        self.current_block = Some(self.blocks.len() - 1);
    }

    /// Called on OSC 133;D;{exit} (command finished)
    pub fn on_command_finish(&mut self, row: usize, exit_code: i32) {
        if let Some(idx) = self.current_block {
            if let Some(block) = self.blocks.get_mut(idx) {
                block.end_row = Some(row);
                block.exit_code = Some(exit_code);
                if let Some(started) = block.started_at {
                    block.duration = Some(started.elapsed());
                }
            }
        }
        self.current_block = None;
    }

    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    pub fn current_block(&self) -> Option<&Block> {
        self.current_block.and_then(|idx| self.blocks.get(idx))
    }

    pub fn prompt_row(&self) -> Option<usize> {
        self.prompt_row
    }

    pub fn command_start_row(&self) -> Option<usize> {
        self.command_start_row
    }

    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// Get the start row of the block before the given absolute row (for Cmd+Up).
    pub fn previous_block_row(&self, current_row: usize) -> Option<usize> {
        self.blocks
            .iter()
            .rev()
            .find(|b| b.start_row < current_row)
            .map(|b| b.start_row)
    }

    /// Get the start row of the block after the given absolute row (for Cmd+Down).
    pub fn next_block_row(&self, current_row: usize) -> Option<usize> {
        self.blocks
            .iter()
            .find(|b| b.start_row > current_row)
            .map(|b| b.start_row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_lifecycle() {
        let mut mgr = BlockManager::new();
        assert!(mgr.is_empty());

        // Prompt start
        mgr.on_prompt_start(0);
        assert_eq!(mgr.prompt_row(), Some(0));

        // Command input start
        mgr.on_command_start(1);
        assert_eq!(mgr.command_start_row(), Some(1));

        // Output start
        mgr.on_output_start(2, "ls -la".to_string());
        assert_eq!(mgr.len(), 1);
        assert!(mgr.current_block().is_some());
        assert_eq!(mgr.current_block().unwrap().command, "ls -la");

        // Command finish
        mgr.on_command_finish(10, 0);
        assert_eq!(mgr.len(), 1);
        assert!(mgr.current_block().is_none());

        let block = &mgr.blocks()[0];
        assert_eq!(block.exit_code, Some(0));
        assert_eq!(block.end_row, Some(10));
        assert!(block.duration.is_some());
    }

    #[test]
    fn test_multiple_blocks() {
        let mut mgr = BlockManager::new();

        mgr.on_prompt_start(0);
        mgr.on_command_start(1);
        mgr.on_output_start(2, "echo hello".to_string());
        mgr.on_command_finish(3, 0);

        mgr.on_prompt_start(4);
        mgr.on_command_start(5);
        mgr.on_output_start(6, "cat file.txt".to_string());
        mgr.on_command_finish(20, 1);

        assert_eq!(mgr.len(), 2);
        assert_eq!(mgr.blocks()[0].id, 0);
        assert_eq!(mgr.blocks()[1].id, 1);
        assert_eq!(mgr.blocks()[1].exit_code, Some(1));
    }

    #[test]
    fn test_block_navigation() {
        let mut mgr = BlockManager::new();

        mgr.on_prompt_start(0);
        mgr.on_command_start(1);
        mgr.on_output_start(2, "echo hello".to_string());
        mgr.on_command_finish(3, 0);

        mgr.on_prompt_start(4);
        mgr.on_command_start(5);
        mgr.on_output_start(10, "cat file.txt".to_string());
        mgr.on_command_finish(20, 0);

        mgr.on_prompt_start(21);
        mgr.on_command_start(22);
        mgr.on_output_start(25, "ls -la".to_string());
        mgr.on_command_finish(30, 0);

        // previous_block_row
        assert_eq!(mgr.previous_block_row(25), Some(10));
        assert_eq!(mgr.previous_block_row(10), Some(2));
        assert_eq!(mgr.previous_block_row(2), None);
        assert_eq!(mgr.previous_block_row(0), None);

        // next_block_row
        assert_eq!(mgr.next_block_row(2), Some(10));
        assert_eq!(mgr.next_block_row(10), Some(25));
        assert_eq!(mgr.next_block_row(25), None);
        assert_eq!(mgr.next_block_row(30), None);
    }
}
