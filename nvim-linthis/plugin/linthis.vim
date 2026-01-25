" linthis.vim - Neovim plugin for linthis
" Maintainer: linthis team
" License: MIT

if exists('g:loaded_linthis')
  finish
endif
let g:loaded_linthis = 1

" Require Neovim 0.9+
if !has('nvim-0.9')
  echohl WarningMsg
  echomsg 'linthis.nvim requires Neovim 0.9 or later'
  echohl None
  finish
endif
