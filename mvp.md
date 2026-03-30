the idea is simple: Kilo, a rust-build (for velocity) terminal text editor, that will be "a son of fresh and neovim", but simpler than both of them. Here is the initial mvp criteria:

- Being able to edit text
- Auto-closing brackets and similars "", '', (), [], {}, <>, ``
- Perfectly done sintax highlighting colors and boldness when necessary, mostly for: **markdown**, **php (laravel, i think it is the same right)**, python, typescript, javascript, css, html, rust, c, sql, git things, .env files, and shell.
- A TOML file for configuration of simple things. (should be at: "~/.config/kilo/kilo.conf.toml")
- Autosave by default, at .5s of delay.
- A pop-up project tree, with nice nerd-icons usage, and git indicators (like: A, U, M.......), and a different thing: each folder will have a color, and the files inside that folder will follow that same color all along. The pop-up should use almost every vertical space of the term screen, and the horizontality should be determined... Also, there should be nice identation between folders and files, so the user is able to determine what is from where, get it? focus really well on the devex.
- A Basic theming system. 
- No clutter on the screen
- Ability to define (on the settings) a padding for the horizontal meaning, so we can do more centralized-screen text/code.

# the **ONLY** BINDS We'll have:

"Insert and Normal mode" = Just like VIM insert and normal mode, but no crazy binds, both modes are literally the same thing, only that in one you cant write.
"ctrl+arrow-keys and arrow-keys" = we'll have NO hjkl for moving, we will use deafult arrow-keys and ctrl+arrow-keys, ok?
"being able to use shift for selecting" = with shift(or ctrl+shift)+arrow-keys
"ctrl+alt+arrow-keys" = for jumping across entire paragraphs.
"ctrl+e" = **IMPORTANT** : pop-up with a tree of the project where i can go on opening and closing folders and select the file I want, and also be able to add (n) a new file, add (f) a new folder, rename (r) anything selected, delete (d) anything selected, move (m) anything selected to inside or out (think REALLY well on how to do this, this is important).
"ctrl+w and ctrl+backspace" = For deleting whole words, as it works on the vscode and other places.
"ctrl+f" = should make a simple sistem of searching through the page the user is in.
"ctrl+shift+f" = should make a search through the entire codebase for what the user searchs. Should be shown as a friendly modal for the user (or not, you shall decide whats better for the DevEx.)
"ctrl+h" = should make it able to do substitution of a desired word/phrase for a desired word/pharse, ok? The changes should be shown one by one for the user to approve.
"ctrl+shift+h" = same thing but for the entire codebase, the changes should be shown one by one for the user to approve. 
"ctrl+z" = undo.
"ctrl+y" = redo.
"ctrl+p" = direct file searching.
"ctrl+," = opens the editor config toml file for quick edits
"ctrl+a" = select all the text on the file

# Stack

Probably simply `rust` with `cross-term` and `ratatui`.