//! How a design inventory set names its documents and directories on disk.

pub const INVENTORY: &str = "inventory.md";
pub const SCREENSHOTS: &str = "screenshots";

#[cfg(test)]
mod tests {
    use crate::inventory::INVENTORY;
    use crate::inventory::SCREENSHOTS;

    #[test]
    fn an_inventory_set_names_its_document_and_screenshots() {
        assert_eq!([INVENTORY, SCREENSHOTS], ["inventory.md", "screenshots"]);
    }
}
