//! How a design inventory set names its documents and directories on disk.

use corpus::set::MemberKind;
use corpus::set::MemberSlot;
use corpus::set::Presentation;
use corpus::set::SetDeclaration;
use corpus::set::SetShape;

pub const INVENTORY: &str = "inventory.md";
pub const SCREENSHOTS: &str = "screenshots";

pub static SET: SetDeclaration = SetDeclaration {
    shape: SetShape {
        root: INVENTORY,
        member_slots: &[MemberSlot {
            pattern: "screenshots/*.png",
            kind: MemberKind::Asset,
        }],
    },
    presentation: Presentation::EntryPoint,
};

#[cfg(test)]
mod tests {
    use corpus::set::MemberKind;
    use corpus::set::Presentation;
    use corpus::set::SetMatcher;
    use corpus::set::SetPlace;
    use corpus::set::SlotPatternError;

    use crate::inventory::INVENTORY;
    use crate::inventory::SCREENSHOTS;
    use crate::inventory::SET;

    #[test]
    fn an_inventory_set_names_its_document_and_screenshots() {
        assert_eq!([INVENTORY, SCREENSHOTS], ["inventory.md", "screenshots"]);
    }

    type Outcome = Result<(), (&'static str, SlotPatternError)>;

    #[test]
    fn an_inventory_set_holds_its_inventory_and_screenshots() -> Outcome {
        let set = SetMatcher::new(&SET)?;
        assert_eq!(set.classify(INVENTORY), Some(SetPlace::Root));
        for path in [
            "screenshots/a.png",
            "screenshots/a.PNG",
            "screenshots/Some File (v2).png",
        ] {
            assert_eq!(
                set.classify(path),
                Some(SetPlace::Member(MemberKind::Asset)),
                "{path}"
            );
        }
        Ok(())
    }

    #[test]
    fn an_inventory_set_holds_no_prototype_or_unslotted_file() -> Outcome {
        let set = SetMatcher::new(&SET)?;
        for path in [
            "prototype-full/x.html",
            "assets/y.svg",
            "notes.txt",
            "screenshots/sub/a.png",
            "Screenshots/a.png",
        ] {
            assert_eq!(set.classify(path), None, "{path}");
        }
        Ok(())
    }

    #[test]
    fn only_the_screenshots_directory_may_hold_members() -> Outcome {
        let set = SetMatcher::new(&SET)?;
        assert!(set.may_contain_members(SCREENSHOTS));
        assert!(!set.may_contain_members("prototype-full"));
        Ok(())
    }

    #[test]
    fn an_inventory_set_is_presented_as_an_entry_point() {
        assert_eq!(SET.presentation, Presentation::EntryPoint);
    }
}
