use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20220101_000001_create_checkpoint_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Checkpoints::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Checkpoints::Idx)
                            .big_unsigned()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Checkpoints::L1Start)
                            .big_unsigned()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Checkpoints::L1End).big_unsigned().not_null())
                    .col(
                        ColumnDef::new(Checkpoints::L2Start)
                            .big_unsigned()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Checkpoints::L2End).big_unsigned().not_null())
                    .col(ColumnDef::new(Checkpoints::CheckpointTxid).string().null())
                    .col(ColumnDef::new(Checkpoints::Status).string().not_null())
                    .to_owned(),
            )
            .await?;

        // manager
        //     .create_index(
        //         Index::create()
        //             .name("idx_checkpoints_l2_blockid")
        //             .table(Checkpoints::Table)
        //             .col(Checkpoints::L2BlockId)
        //             .to_owned(),
        //     )
        //     .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_checkpoints_idx")
                    .table(Checkpoints::Table)
                    .col(Checkpoints::Idx)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(Index::drop().name("idx_checkpoints_l2_blockid").to_owned())
            .await?;

        manager
            .drop_index(Index::drop().name("idx_checkpoints_idx").to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Checkpoints::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum Checkpoints {
    Table,
    Idx,
    L1Start,
    L1End,
    L2Start,
    L2End,
    CheckpointTxid,
    Status,
}
