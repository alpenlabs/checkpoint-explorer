use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Blocks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Blocks::BlockHash)
                            .string()
                            .not_null()
                            .unique_key(), // Unique key for block_hash
                    )
                    .col(
                        ColumnDef::new(Blocks::Height)
                            .big_unsigned()
                            .not_null()
                            .unique_key(), // Unique key for height
                    )
                    .col(
                        ColumnDef::new(Blocks::CheckpointIdx)
                            .big_unsigned()
                            .not_null(), // Foreign key column
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_blocks_checkpoint_idx")
                            .from(Blocks::Table, Blocks::CheckpointIdx)
                            .to(Checkpoints::Table, Checkpoints::Idx),
                    )
                    .to_owned(),
            )
            .await?;

        // Create separate indexes for block_hash and height for fast queries
        manager
            .create_index(
                Index::create()
                    .name("idx_blocks_block_hash")
                    .table(Blocks::Table)
                    .col(Blocks::BlockHash)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_blocks_height")
                    .table(Blocks::Table)
                    .col(Blocks::Height)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(Index::drop().name("idx_blocks_block_hash").to_owned())
            .await?;

        manager
            .drop_index(Index::drop().name("idx_blocks_height").to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Blocks::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Blocks {
    Table,
    BlockHash,
    Height,
    CheckpointIdx,
}

#[derive(DeriveIden)]
enum Checkpoints {
    Table,
    Idx,
}
