use std::path::Path;

use git2::{Object, Oid, Repository};

pub struct Session<S> {
    repository: Repository,
    sender: S,
}

impl<S> Session<S>
where
    S: FnMut(ServerCommand),
{
    pub fn new<P: AsRef<Path>>(path: P, sender: S) -> Result<Self, git2::Error> {
        Ok(Self {
            repository: Repository::open(path)?,
            sender,
        })
    }

    pub fn handle(&mut self, command: ClientCommand) {
        let ClientCommand::Want(query) = command;
        self.handle_want(query)
    }

    fn handle_want(&mut self, query: WantQuery) {
        let base = self.repository.find_object(query.base(), None).unwrap();
        (self.sender)(ServerCommand::Push(&base));

        match query {
            WantQuery::CommitAncestry(_, commit_ancestry) => {
                if commit_ancestry > 0 {
                    let parent_ids = base.as_commit().unwrap().parent_ids().collect::<Vec<_>>();
                    drop(base);
                    for parent_id in parent_ids {
                        self.handle_want(WantQuery::CommitAncestry(parent_id, commit_ancestry - 1));
                    }
                }
            }
            WantQuery::PeelTree(_) => todo!(),
            WantQuery::PeelBlob(_) => todo!(),
        }
    }
}

pub enum ClientCommand {
    Want(WantQuery),
}

pub enum WantQuery {
    CommitAncestry(Oid, usize),
    PeelTree(Oid),
    PeelBlob(Oid),
}

impl WantQuery {
    pub fn object(oid: Oid) -> Self {
        Self::CommitAncestry(oid, 0)
    }

    fn base(&self) -> Oid {
        match *self {
            WantQuery::CommitAncestry(base, _) => base,
            WantQuery::PeelTree(base) => base,
            WantQuery::PeelBlob(base) => base,
        }
    }
}

#[derive(Clone)]
pub enum ServerCommand<'r, 'o> {
    Push(&'o Object<'r>),
}

#[cfg(test)]
mod test {
    use git2::Oid;

    use crate::{ClientCommand, ServerCommand, Session, WantQuery};

    #[test]
    fn pushes_wanted_object() {
        let mut down = vec![];
        let mut session = Session::new("..", |ServerCommand::Push(obj)| {
            down.push(obj.as_commit().unwrap().summary().unwrap().to_string())
        })
        .unwrap();
        let up = ClientCommand::Want(WantQuery::object(
            Oid::from_str("47877e8822fa32cce2580089990623eb2bd59363").unwrap(),
        ));
        session.handle(up);

        assert_eq!(&down[0], "commit message");
    }
    #[test]
    fn pushes_wanted_parent() {
        let mut down = vec![];
        let mut session = Session::new("..", |ServerCommand::Push(obj)| {
            down.push(obj.as_commit().unwrap().summary().unwrap().to_string())
        })
        .unwrap();
        let up = ClientCommand::Want(WantQuery::CommitAncestry(
            Oid::from_str("f80dbc1436db9aec14bc7be79d0d21d0d132d5fe").unwrap(),
            1,
        ));
        session.handle(up);

        assert_eq!(&down[0], "commit 2");
        assert_eq!(&down[1], "commit message");
    }

    #[test]
    fn pushes_wanted_parents() {
        let mut down = vec![];
        let mut session = Session::new("..", |ServerCommand::Push(obj)| {
            down.push(obj.as_commit().unwrap().summary().unwrap().to_string())
        })
        .unwrap();
        let up = ClientCommand::Want(WantQuery::CommitAncestry(
            Oid::from_str("3b6be088f09c77ff62ae9edcde714c7ca9733b49").unwrap(),
            1,
        ));
        session.handle(up);

        assert_eq!(&down[0], "Merge branch 'test' into HEAD");
        assert_eq!(&down[1], "commit in other branch");
        assert_eq!(&down[2], "commit 2");
    }

    fn ignores_have_commits() {}

    fn pushes_tree() {}

    fn pushes_blob() {}
}
