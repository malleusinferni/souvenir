# Souvenir

This nasty little parasite of a language is a project to combine features of [Ink][ink] and [Erlang][erlang] in a single interpreter for embedding in a game engine. Here's a quick sample:

[ink]: https://github.com/inkle/ink
[erlang]: http://www.erlang.org

```souvenir
== start
> Howdy. What kind of language are you looking for?

weave
| > A language for making games
    > Great! That's exactly what we made this for.
| > A language with concurrency and/or screaming robots
    > Well, we have that stuff too! Check this out.
    wait 10
    let ScreamingRobot = spawn scream(9000)
    wait 30
    > ...Yeah, this might go on for a while.
;;

> Anyway, keep an eye on this space. Better documentation
> and some weird but cool features are on the way!

== scream(Count)
> Robot: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"

-- FIXME: Wait a second. This is an infinite loop!
-> scream(Count - 1)
```

Souvenir was created to meet the needs of a particular game, so some of the features might seem like they're out of left field. However, the interpreter is designed to be embeddable into more or less arbitrary game engines. Eventually it will even be possible to compile your scripts to bytecode ahead of time and write a custom backend for, say, Unity or Unreal. I say "eventually" because none of this is production-ready at the moment -- but feel free to ask questions or make feature requests.
