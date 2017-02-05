" Vim syntax file
" Language:     Souvenir
" Repository:   https://github.com/malleusinferni/souvenir/

if version < 600
  syntax clear
elseif exists('b:current_syntax')
  finish
endif

syn match souvenirNumber /\<\d\+\>/
syn match souvenirRandom /\<\d\+d\d\+\>/
syn match souvenirArithmetic /-\|+\|\*\|<=\?/

syn region souvenirString start=/> / end=/$/ contains=souvenirTemplate
syn region souvenirComment start=/--/ end=/$/

syn region souvenirTemplate matchgroup=souvenirTemplateMarker start=/{{/ matchgroup=souvenirTemplateMarker end=/}}/ contained contains=souvenirVariableName

syn keyword souvenirMatch trap given listen weave branch when if then
syn keyword souvenirCommand let trace wait disarm spawn
syn keyword souvenirKeyword from

" Must come before SceneDef
syn match souvenirEquals /=/

syn match souvenirSceneDef /^==/ nextgroup=souvenirSceneName skipwhite
syn match souvenirSceneName /\<\l\w*/ nextgroup=souvenirSceneArgs "contained
syn region souvenirSceneArgs matchgroup=souvenirParen start=/(/ matchgroup=souvenirParen end=/)/ contained contains=souvenirAtom,souvenirNumber,souvenirVariableName

syn match souvenirModName /\<\w\+:/he=e-1,me=e-1 nextgroup=souvenirModSep
syn match souvenirModSep /\>:\</ nextgroup=souvenirSceneName
syn match souvenirDivert /->/ nextgroup=souvenirSceneName,souvenirModName skipwhite
syn match souvenirLabel /'\l\w*/
syn match souvenirMacro /?\u[A-Z0-9_]*/
syn match souvenirAtom /#\l[a-z0-9_]*/
syn match souvenirVariableName /\u[A-Za-z]*/
syn match souvenirChoice /|/
syn match souvenirSend /<-/
syn match souvenirEnd /;;/

syn keyword souvenirSpecialVar Self _

hi def link souvenirChoice Label
hi def link souvenirDivert Statement

hi def link souvenirString String
hi def link souvenirNumber Number
hi def link souvenirRandom Number
hi def link souvenirSpecialVar Special

hi def link souvenirEquals Statement
hi def link souvenirComment Comment
hi def link souvenirArithmetic Operator
hi def link souvenirMatch Conditional
hi def link souvenirCommand Statement
hi def link souvenirKeyword Keyword

hi def link souvenirModName PreProc
hi def link souvenirModSep Delimiter
hi def link souvenirSceneDef PreProc
hi def link souvenirMacro Macro
hi def link souvenirSceneName Function
hi def link souvenirLabel Tag
hi def link souvenirParen Delimiter
hi def link souvenirTemplateMarker Delimiter
hi def link souvenirAtom Constant
hi def link souvenirVariableName Identifier
hi def link souvenirSend Statement
hi def link souvenirEnd Delimiter

let b:current_syntax = 'souvenir'
