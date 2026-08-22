import { mount } from 'svelte'
import App from './App.svelte'
import './style.css'
import './macos27.css'
import { installLiquidGlass } from './lib/liquid-glass'

mount(App, { target: document.getElementById('app')! })
installLiquidGlass()
