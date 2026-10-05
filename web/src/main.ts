import { mount } from 'svelte';
import App from './App.svelte';
import './tailwind.css';
import { applyTheme, savedTheme } from './theme';

applyTheme(savedTheme());

mount(App, { target: document.getElementById('app')! });
