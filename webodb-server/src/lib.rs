use std::{path::Path, iter};

use git2::{Object, Oid, Repository};

struct Session {
    repository: Repository,
}

impl Session {
    fn new<P: AsRef<Path>>(path: P) -> Result<Self, git2::Error> {
        Ok(Self {
            repository: Repository::open(path)?,
        })
    }

    fn handle(&mut self, command: ClientCommand) -> impl Iterator<Item = ServerCommand<'_>> + '_ {
        let ClientCommand::Want(query) = command;

        WantQueryResolver {
            query,
            repository: &self.repository,
            last_object: None,
        }.map(ServerCommand::Push)
    }
}

/// something like this for the iterator which will complete the query
struct WantQueryResolver<'r> {
    query: WantQuery,
    repository: &'r Repository,
    last_object: Option<Oid>,
}

impl<'r> Iterator for WantQueryResolver<'r> {
    type Item = Object<'r>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(last_object) = &mut self.last_object {
            let WantQuery {
                base: _,
                commit_ancestry,
                tree,
                blob,
            } = &mut self.query;
            if *commit_ancestry > 0 {
                *commit_ancestry -= 1;
                let object = self.repository.find_commit(*last_object).unwrap();
                let object = object.parent(0).unwrap(); // complicated - need to fan out and iterate up each of the parents, same for tree
                self.last_object = Some(object.id());
                Some(object.into_object())
            } else {
                todo!()
            }
        } else {
            let object = self.repository.find_object(self.query.base, None).unwrap();
            self.last_object = Some(object.id());
            Some(object)
        }
    }
}

enum ClientCommand {
    Want(WantQuery),
}

struct WantQuery {
    base: Oid,
    commit_ancestry: usize,
    tree: bool,
    blob: bool,
}

impl WantQuery {
    fn oid(oid: Oid) -> Self {
        Self {
            base: oid,
            commit_ancestry: 0,
            tree: false,
            blob: false,
        }
    }

    fn ancestor(self, depth: usize) -> Self {
        Self {
            commit_ancestry: depth,
            ..self
        }
    }

    fn tree(self) -> Self {
        Self {
            tree: true,
            ..self
        }
    }

    fn blob(self) -> Self {
        Self {
            blob: true,
            ..self
        }
    }
}

enum ServerCommand<'r> {
    Push(Object<'r>),
}

#[cfg(test)]
mod test {
    use git2::{Object, Oid};

    use crate::{ClientCommand, ServerCommand, Session, WantQuery};

    #[test]
    fn pushes_wanted_object() {
        let mut session = Session::new("..").unwrap();
        let up = ClientCommand::Want(WantQuery::oid(Oid::from_str("47877e8822fa32cce2580089990623eb2bd59363").unwrap()));
        let mut down = session.handle(up);
        
        let ServerCommand::Push(obj1) = down.next().unwrap();
        let commit = obj1.as_commit().unwrap();
        assert_eq!(commit.summary(), Some("commit message"));
    }

    #[test]
    fn pushes_wanted_parents() {
        let mut session = Session::new("..").unwrap();
        let up = ClientCommand::Want(WantQuery::oid(Oid::from_str("f80dbc1436db9aec14bc7be79d0d21d0d132d5fe").unwrap()).ancestor(1));
        let mut down = session.handle(up);
        
        let ServerCommand::Push(obj) = down.next().unwrap();
        let commit = obj.as_commit().unwrap();
        assert_eq!(commit.summary(), Some("commit 2"));
        let ServerCommand::Push(obj) = down.next().unwrap();
        let commit = obj.as_commit().unwrap();
        assert_eq!(commit.summary(), Some("commit message"));
    }

    fn ignores_have_commits() {}

    fn pushes_tree() {}

    fn pushes_blob() {}
}