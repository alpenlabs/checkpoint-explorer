use sea_orm::Order;

pub(crate) fn resolve_order(order: Option<&str>) -> Order {
    match order {
        Some("asc") => Order::Asc,
        Some("desc") => Order::Desc,
        _ => Order::Desc,
    }
}
