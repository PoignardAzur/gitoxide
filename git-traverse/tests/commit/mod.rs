mod ancestor {
    use crate::hex_to_id;
    use git_hash::ObjectId;
    use git_odb::{linked::Db, pack, FindExt};
    use git_traverse::commit;

    fn db() -> crate::Result<Db> {
        let dir = git_testtools::scripted_fixture_repo_read_only("make_traversal_repo_for_commits.sh")?;
        let db = Db::at(dir.join(".git").join("objects"))?;
        Ok(db)
    }

    fn new_iter(
        tips: impl IntoIterator<Item = impl Into<ObjectId>>,
    ) -> impl Iterator<Item = Result<ObjectId, commit::ancestors::Error>> {
        let db = db().expect("db instantiation works as its definitely valid");
        commit::Ancestors::new(tips, commit::ancestors::State::default(), move |oid, buf| {
            db.find_existing_commit_iter(oid, buf, &mut pack::cache::Never).ok()
        })
    }

    fn check_traversal_with_shared_reference(tips: &[&str], expected: &[&str]) -> crate::Result {
        let tips: Vec<_> = tips.iter().copied().map(hex_to_id).collect();
        let oids: Result<Vec<_>, _> = new_iter(tips.iter().cloned()).collect();
        let expected: Vec<_> = tips
            .into_iter()
            .chain(expected.iter().map(|hex_id| hex_to_id(hex_id)))
            .collect();
        assert_eq!(oids?, expected);
        Ok(())
    }

    #[test]
    fn instantiate_with_arc() -> crate::Result {
        let _ = new_iter(vec![git_hash::ObjectId::null_sha1()]);
        Ok(())
    }

    #[test]
    fn instantiate_with_box() -> crate::Result {
        let _ = new_iter(vec![git_hash::ObjectId::null_sha1()]);
        Ok(())
    }

    #[test]
    fn linear_history_no_branch() -> crate::Result {
        check_traversal_with_shared_reference(
            &["9556057aee5abb06912922e9f26c46386a816822"],
            &[
                "17d78c64cef6c33a10a604573fd2c429e477fd63",
                "9902e3c3e8f0c569b4ab295ddf473e6de763e1e7",
                "134385f6d781b7e97062102c6a483440bfda2a03",
            ],
        )
    }

    #[test]
    fn simple_branch_with_merge() -> crate::Result {
        check_traversal_with_shared_reference(
            &["01ec18a3ebf2855708ad3c9d244306bc1fae3e9b"],
            &[
                "efd9a841189668f1bab5b8ebade9cd0a1b139a37",
                "ce2e8ffaa9608a26f7b21afc1db89cadb54fd353",
                "9556057aee5abb06912922e9f26c46386a816822",
                "9152eeee2328073cf23dcf8e90c949170b711659",
                "17d78c64cef6c33a10a604573fd2c429e477fd63",
                "9902e3c3e8f0c569b4ab295ddf473e6de763e1e7",
                "134385f6d781b7e97062102c6a483440bfda2a03",
            ],
        )
    }

    #[test]
    fn multiple_tips() -> crate::Result {
        check_traversal_with_shared_reference(
            &[
                "01ec18a3ebf2855708ad3c9d244306bc1fae3e9b",
                "9556057aee5abb06912922e9f26c46386a816822",
            ],
            &[
                "efd9a841189668f1bab5b8ebade9cd0a1b139a37",
                "ce2e8ffaa9608a26f7b21afc1db89cadb54fd353",
                "17d78c64cef6c33a10a604573fd2c429e477fd63",
                "9152eeee2328073cf23dcf8e90c949170b711659",
                "9902e3c3e8f0c569b4ab295ddf473e6de763e1e7",
                "134385f6d781b7e97062102c6a483440bfda2a03",
            ],
        )
    }
}
