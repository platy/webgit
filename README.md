# webgit

The origin of this idea is that I want to make a new version of the gitgov app, I'm leaning towards this version being written in pure rust without any heavy frameworks. The new version needs filtering, a local data sync and generally should be fast as fuck. The old version used the github api and no backend and I was thinking the new version needs a specific backend, but now I'm thinking that I can make a generic git sync protocol that would do the trick for this and perhaps some other things. This gives some more constraints to the protocol design and means that it would be able to handle anything that i can represent in git in the future. Also it would be interesting to make a really snappy git browser using it and it might open up some possibilities other apps.

Options:
* Make/Find a client for the [git http protocol](https://www.git-scm.com/docs/http-protocol) with this I don't even need to design a protocol and it would be able to use any git server as the backend. Cons: it's about fetching individual resources or whole trees & I'm not sure if the smart api will work from the browser.
* Build a new protocol which can do thins more effectively for my use case (including pushing updates and synchronising a filtered commit log)
* Have the new protocol work as an extension to the git dumb protocol like the git-pack ones do. Then I can reuse the parts that work for what I'm doing and just add the new stuff.

I'm not going to really know how much of the git http protocols are applicable to my use case or what I can learn from them without either a much better explanation than the one above, or writing an implementation. So maybe that is the start point, also it will give me license to call it webgit if it implements git and I just add an extension on top.

The other approach was that I was going to start off by making a remote syncing odb implemtation allowing the server to push objects it wasn't asked for - in the end it doesn't seem that there would be much crossover with the git protocol which is about syncing the whole history of some refs. I really want to do more of a graphQL-like protocol.

## The protocol design

### ODB partial sync

All of the objects need to be stored on the server, the client will have a subset of what the server has. The sync design is about maintaining a smart cache on the client so that objects are accessible without delay as much as possible.

The server stores:
* a map of OID to Object for the resource. (Though that's in the git repo)
* a set of OIDs for each connection that exist on the other side (perhaps there is some information about each to store too? - but maybe not)

The client stores:
* a map of the OIDs it has
* a set of OIDs it has requested and is waiting for
* a queue of OIDs that it wants and hasn't requested

#### Messages

C->S have(oid) : something the client has
C->S want(query) : something the client wants
    Support revs and a graph query syntax, eg. (oid)^3.tree.blob or similar. The server must make sure every object it touches is pushed, so the 3 nearest ancestors of oid as well as that last ancestor's tree and all its blobs. Got to see if this makes sense.

S->C push(object) : an object for the client to cache
S->C ????(oid) : the object doesn't exist

#### ODB client library

With this I should be able to implement a wasm library implementing the git2 Odb interface (but async) to access a cache stored in the browser (perhaps using indexdb) and using this protocol to fill any cache misses.

#### ODB queries

Peel is some kind of existing git way to do what I'm looking for.

Query = Oid [ Commit-Query ] | [ Tree-Query ]
Commit-Query = [ Ancestor-Ref ] [ .tree ]
Tree-Query = .blobs | .tree : only valid if the preceding part of the query refers to a tree, resolves to the whole tree and (with .blobs) all of the referred blobs

Oid [^ancestor number]

### Read-only bare git

For read-only git without any working copy or indexes, it doesn't seem there is that much needed:
* refs
* parsing commit messages
* iterating over trees
* revwalk

#### Messages

S->C refs(oid, refname)* : The server sends or pushes updates of refs - no need to make the client request these
C->S revwalk(push-ish*, ignore-ish*, query) : The client requests that the server do the query on all commits from the revwalk (pushing everything it touches.

#### Git read client library

The client stores revs, provides an api for getting and inspecting commits, using commit-ishs, doing a revwalk, tree-diff.

### extension: Commit search

This one may be more like the reactive query like with the transit radar. Client and server both perform the same search. If possible it would work sort of similar to revwalk and may even be an extension of revwalk, it needs to be able to limit the number of results.

Search commit message terms, date, paths changed

### extension: Blob search

Search inside all blobs in a history, this would then also need to push all the commits which changed those blobs. The number of results would need to be limited somehow.

### extension: File/Tree history

By specifying a path in a commit, find the history of changes of that path.
