# Word based diff

It's not used as replacement for storing differences in files, but to pretty-print differences in the files.
It's scoring is ignoring whitespaces (apart from indentation) and it ignores whitespace in the left document when showing diff.
So in effect, it displays right document, and annotate differences there.

## Integration with git

Add this into .gitconfig

```
[alias]
  piff = !git difftool --tool=piff --no-prompt
[difftool "piff"]
  cmd = /home/mic/.local/bin/platypus-diff "$LOCAL" "$REMOTE"
```

## TODO

- [ ] Show limited context by default.
- [ ] Integration with neovim? (if possible).
- [ ] Make it faster on almost identical documents (i.e. when showing diff).
- [ ] Include syntax highlighting (i.e. using bat).
