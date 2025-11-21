# toy-hashgraph

An implementation of the hashgraph protocol as a library.

## Idea
1. Every peer instantiates a `Hashgraph` object
2. After a few ticks, you call `gossip` to know who to sync with
3. Every time someone syncs with you, you forward it to the object with `recieve`

