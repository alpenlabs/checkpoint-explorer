use sea_orm_migration::{prelude::*, sea_orm::ConnectionTrait};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Checkpoints::Table)
                    .modify_column(ColumnDef::new(Checkpoints::L2Start).big_unsigned().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("UPDATE checkpoints SET l2_start = l2_end WHERE l2_start IS NULL")
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Checkpoints::Table)
                    .modify_column(
                        ColumnDef::new(Checkpoints::L2Start)
                            .big_unsigned()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Checkpoints {
    Table,
    L2Start,
}
