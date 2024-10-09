# Succinct data structures
Welcome to a new blog series about succinct data structures!

I'm currently revisiting a lot of the data structures we originally
wrote for TerminusDB, and rewriting them to take advantage of vector
parallelization strategies (SIMD). Since that means my brain is going
to be filled with the details of all these structures, this is a great
time for me to document this stuff.

My intent is to write a series of articles, discussing each data
structure in its own article, as well as going over some ways that we
can actually use these data structures in practice. Let's begin!

## What are succinct data structures?
To put it succinctly, succinct data structures are compressed data
structures that retain useful search characteristics. When compressing
data, we can really go extreme, spending a lot of cpu cycles to really
calculate something very close to some theoretic minimum for lossless
compression. But the result of this compression is generally
completely unusable without first decompressing.

As a simple example, imagine a long array of integers. Due to how
computer registers and memory work, there are a few natural options
for storing this array in memory. Depending on our expected maximum
element size, we can make this array hold 8, 16, 32 or 64 bit values.

Suppose our integers are all between 0 and 1000. 8 bits aren't enough
to hold all these values (as the maximum value it can hold is
255). Our next option is 16 bit, which raises the upper limit
to 65535. This however means there's 64535 values that could be
stored, but which will never actually occur. Specifically, if we look
at each 16 bit value, we'll see that the first 6 bits are always 0!

Compression algorithms are very good at getting rid of repeating bit
patterns like that. But the moment you use a regular compression
algorithm, the data structure is no longer really usable as an array.

Instead of going for a full compression algorithm, there's something
much simpler we can do here. As we observed above, for the 0-1000
range array, 6 out of 16 bits go unused in every 16 bit entry. So why
not just get rid of those 6 unused bits, and keep only the 10 that are
actually being used? This is the basic idea behind the first succinct
data structure to discuss in this series, the logarray.

## succinct or compressed?
Before going on, one thing to note here before I get angry e-mails
saying I'm wrong is that I might not always use 'succinct'
correctly. In practical terms, 'succinct' to me means 'data structure
that is compressed but still efficient to use'. Actually though,
there's a much more formal definition of 'succinct data structure'
which says that a data structure is succinct only if its format is
close to the information-theoretic minimum, while still retaining
desired access characteristics.

If a structure is smaller than the original but not quite that small,
an information theorist might say that it is compressed, but not
succinct. I however am no information theorist and couldn't tell you
how to calculate the information-theoretic minimum of most
things. Furthermore, to me 'compressed' could also mean something you
ran the data through gzip, not necessarily that it is actually usable
as a data structure. For those reasons, I'll keep using 'succinct'
even when formally it might be better to say 'compressed'.

With that caveat out of the way, lets move on to the logarray.

## The logarray
A logarray is our name for a bit-packed array, where each array
element takes only as many bits as the largest element we expect. So
for our example of a maximum of 1000 above, that is 10 bit (since this
maximally expresses 1023). For 100, it would be 7 bits (which lets you
express to 127).

To be honest, I don't remember why we decided to call it a
logarray. This might be directly lifted from one of the HDT papers.
Maybe it relates to how the amount of bits you need is a log2
(rounded up) of the exclusive upper bound. Whatever the case, this is
what I'll keep calling it.

In order to compare this scheme with a regular array, let's go over
some things that make regular arrays nice, and see how this logarray
compares.

### Constant time lookup
The most important property of an array is the ability to look up
elements in it at constant time. This means that, no matter whether we
look up element 5 or element 5 million, we can expect the operation to
take about the same amount of time. More formally, there's a constant
upper bound to the runtime. In computer science, this is usually
written as `O(1)` (also known as Big O notation).

Arrays can do this because each element has a fixed size and
alignment, so the memory location that a particular element lives at,
its address, can be easily calculated from the start and an
offset. CPUs generally have dedicated instructions for working with
memory like this, so it is very efficient.

Compare this with another list-like structure, the linked list. In a
linked list, elements don't live packed together, but could live
anywhere in memory. To find the element at a particular index, you
need to follow the links between these elements until you arrive at
the right location. So if you need element 1, you'll only need to do
one hop from element 0, but if you need element 1000, you're going to
have to do one thousand hops. So unlike arrays, where these lookups
are constant no matter what the address is, in the case of a linked
list, the lookup time grows linearly with the magnitude of our
input. In Big O notation, we'd say this is `O(n)`.

Constant time lookup is very nice. It allows algorithms operating on
the array to rely on efficient random access. Abandoning this property
effectively means a big group of useful algorithms become impossible
to do well. For example, all efficient sorting algorithms rely on
random access.

Luckily, logarrays do support constant-time data access. While there's
more work that needs to be done for each data access (to deal with the
compression and decompression), the time this takes does not depend on
the element index at all.

### Memory locality
Another nice property of arrays is memory locality. This means that
all the array data lives packed together in memory, rather than being
spread out. The reason that this is important is because modern CPUs
are much faster at dealing with data that is close to each other than
data that is further apart. So the closer you can get data together,
the faster a cpu can actually use that data.

Specifically, most modern CPUs cache memory at a minimum granularity
of 64 bytes. This means that, whenever a program reads a particular
location in memory, the CPU actually fetches a chunk of 64 bytes from
memory at once. If the program subsequently needs a value in that
exact same chunk, it'll already be in cache and load much
faster. Additionally, modern CPUs often implement various memory
prefetching strategies, where the CPU predicts that a memory access is
likely going to happen soon, so it already loads the required memory
into cache before the actual instruction is given.

This hardware support ensures that a sequential scan over an array is
very fast. Also, for algorithms that can work on one chunk of the
array at a time, it's often possible to pick a chunk size such that
all data accesses for that chunk happen within a typical cpu cache
capacity.

Losing this property is less catastrophic than losing constant data
access. We remain `O(1)`, it's just that our upper bound for element
 access is much bigger.

That said, for the logarray, we're actually improving data locality!
We manage to fit more array elements in the same amount of
memory after all, so the density goes up. This means work can be done
on more elements at once.

### Hardware support for numerical types and arithmetic
While memory locality's influence on caching may seem a bit subtle and
behind the scenes, there's another element of hardware support for
arrays that is a lot more obvious. This is the fact that CPUs have a
range of natively supported numerical types.

Remember that we figured out that we could store integers between 0
and 1000 in just 10 bits? That is all well and good, but I don't know
of any modern CPU that actually is able to do 10-bit arithmetic
natively, or decode them from a packed sequence. In order to do any
sort of calculations, such compressed numbers first need to be
decompressed in software, so we can actually put them into registers
and do some math.

for calculation-heavy problems, this compression and decompression
overhead adds a lot of processing time that is therefore not going
into actually working out the problem itself.

### Library support
There's a wide variety of libraries out there that will happily take a
slice of memory to work on, but won't take some weird custom logarray
type. For example, the rust standard library comes with two different
sorting algorithms (timsort and pdqsort), which operate on memory
slices, but simply won't work on collection types that operate
differently.

Having your data as a plain slice of memory pretty much guarantees
easy interopability, and moving away from plain slices of memory means
losing out on a lot of functionality you'd otherwise get out of the
box. While in the long run anything can be reimplemented, losing out
on a lot of functionality that is regularly taken for granted is a
significant cost to any software development project.

## Evaluation of the logarray
In terms of the type of algorithms theoretically possible, there is
nothing that a regular array can do that a logarray can't. It is
usable like an array, except that it is slower. In exchange for this
performance loss, and the loss in programmer ergonomics due to losing
software library interopability, you get to fit more data in the same
amount of memory. Yay!

So is this worth it? I'd say that generally it is not. If you have the
memory to spare, using uncompressed data structures is most likely
going to perform much better.

But we don't always have the choice. For certain big data analytics
problems, squeezing more data into the same amount of memory can avoid
a costly upgrade or cloud compute bill, or avoid having to develop a
distributed computing strategy. On the other end of the power
spectrum, we might be dealing with some embedded device that is
heavily memory or storage restricted, requiring us to pack as much as
we can in what we have.

Even for problems that are neither big nor small, the ability to
squeeze more data into the CPU cache might offset the
compression/decompression cost, especially for certain carefully
chosen sizes which we can compress and decompress more
quickly. However, whether or not this makes sense for a particular
problem requires careful profiling, and writing cache-friendly code is
far from trivial.

## Why do we use succinct data structures?
After having told you why you should generally not use them, I should
probably explain why we do. In a way this is just a historical
decision based on adapting the HDT papers to our work. HDT (short for
Header, Dictionary, Triple) is a storage format for RDF data built out
of succinct data structures. Furthermore, this format is widely used
with a particular indexing strategy which is also built up around
succinct data structures.

Originally, TerminusDB was prototyped around HDT, where data layers
were set up as addition and removal graphs serialized into their own
hdt files. When I rewrote the original storage layer, these data
structures were simply adapted slightly to better fit the use case of
incrementally growing databases.

In practice though, we found these structures to work remarkably well
for our use case of graph search. The achieved data density for graph
data is amazing, and while I know for a fact the existing
implementation of these data structures could be better (that's why I
am doing a rewrite!), their performance has generally not been a
bottleneck in TerminusDB as a whole.

For our problems, we decided that the performance was good enough and
that the memory savings were worth it.

## Going forward
Now that we got a clearer understanding of succinct data structures,
what they're good for, and what tradeoff is being made, we are ready
to look at some low-level details.

In the next posts in this series, we'll explore various succinct data
structures, some quirks around their implementation, and ways to
combine them into composite data structures. Probably we'll even
implement some custom graph format at some point!

Stay tuned..
