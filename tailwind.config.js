/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
        mono: ['"JetBrains Mono"', 'ui-monospace', 'SFMono-Regular', 'monospace'],
      },
      colors: {
        // Primary brand — vivid indigo → violet
        brand: {
          50:  '#eef1ff',
          100: '#e0e5ff',
          200: '#c6ccff',
          300: '#a3a8ff',
          400: '#8482ff',
          500: '#6d5cff',
          600: '#5b3df5',
          700: '#4c2ddb',
          800: '#3f28b0',
          900: '#35268b',
        },
        // Secondary accent — electric cyan/teal
        accent: {
          50:  '#eafffb',
          100: '#cdfff4',
          200: '#9ffdea',
          300: '#5ff6dc',
          400: '#22e6c8',
          500: '#06c9ae',
          600: '#02a48f',
          700: '#078274',
          800: '#0c675e',
          900: '#0f544e',
        },
        // Tertiary accent — warm magenta/pink for highlights
        magenta: {
          400: '#ff6ac1',
          500: '#f43fb0',
          600: '#db1f96',
        },
        // Recording / live signal colors
        signal: {
          rec:   '#ff4d6d',
          trans: '#ffb02e',
          idle:  '#8b8ba7',
        },
        // Deep, slightly blue-tinted dark surfaces
        surface: {
          950: '#080810',
          900: '#0c0c17',
          800: '#13131f',
          700: '#1b1b2b',
          600: '#242438',
          500: '#2f2f47',
          400: '#3b3b57',
        },
      },
      backgroundImage: {
        'brand-gradient':  'linear-gradient(135deg, #6d5cff 0%, #8482ff 40%, #22e6c8 100%)',
        'brand-sheen':     'linear-gradient(135deg, #6d5cff 0%, #4c2ddb 100%)',
        'aurora':          'radial-gradient(60% 55% at 15% 10%, rgba(109,92,255,0.30) 0%, transparent 60%), radial-gradient(55% 50% at 85% 20%, rgba(34,230,200,0.22) 0%, transparent 60%), radial-gradient(70% 60% at 50% 100%, rgba(244,63,176,0.18) 0%, transparent 65%)',
        'rec-gradient':    'linear-gradient(135deg, #ff4d6d 0%, #f43fb0 100%)',
        'trans-gradient':  'linear-gradient(135deg, #ffb02e 0%, #ff7b54 100%)',
        'idle-gradient':   'linear-gradient(135deg, #5b3df5 0%, #06c9ae 100%)',
      },
      boxShadow: {
        'glow-brand':  '0 0 0 1px rgba(109,92,255,0.35), 0 8px 30px -6px rgba(109,92,255,0.55)',
        'glow-accent': '0 0 0 1px rgba(34,230,200,0.35), 0 8px 30px -6px rgba(34,230,200,0.45)',
        'glow-rec':    '0 0 0 1px rgba(255,77,109,0.40), 0 8px 32px -4px rgba(255,77,109,0.55)',
        'soft':        '0 10px 40px -12px rgba(0,0,0,0.7)',
      },
      keyframes: {
        'aurora-drift': {
          '0%, 100%': { transform: 'translate3d(0,0,0) scale(1)' },
          '50%':      { transform: 'translate3d(0,-3%,0) scale(1.08)' },
        },
        'shimmer': {
          '0%':   { backgroundPosition: '-200% 0' },
          '100%': { backgroundPosition: '200% 0' },
        },
        'float-up': {
          '0%':   { opacity: '0', transform: 'translateY(8px)' },
          '100%': { opacity: '1', transform: 'translateY(0)' },
        },
        'pop-in': {
          '0%':   { opacity: '0', transform: 'scale(0.96)' },
          '100%': { opacity: '1', transform: 'scale(1)' },
        },
        'ring-pulse': {
          '0%':   { transform: 'scale(0.9)', opacity: '0.7' },
          '70%':  { transform: 'scale(2.2)', opacity: '0' },
          '100%': { transform: 'scale(2.2)', opacity: '0' },
        },
        'wave': {
          '0%, 100%': { transform: 'scaleY(0.35)' },
          '50%':      { transform: 'scaleY(1)' },
        },
        'glow-breathe': {
          '0%, 100%': { opacity: '0.55' },
          '50%':      { opacity: '1' },
        },
        'gradient-pan': {
          '0%, 100%': { backgroundPosition: '0% 50%' },
          '50%':      { backgroundPosition: '100% 50%' },
        },
        'checkmark': {
          '0%':   { transform: 'scale(0) rotate(-20deg)', opacity: '0' },
          '60%':  { transform: 'scale(1.2) rotate(0deg)', opacity: '1' },
          '100%': { transform: 'scale(1) rotate(0deg)', opacity: '1' },
        },
      },
      animation: {
        'aurora-drift':  'aurora-drift 14s ease-in-out infinite',
        'shimmer':       'shimmer 2.2s linear infinite',
        'float-up':      'float-up 0.4s cubic-bezier(0.22, 1, 0.36, 1) both',
        'pop-in':        'pop-in 0.28s cubic-bezier(0.22, 1, 0.36, 1) both',
        'ring-pulse':    'ring-pulse 1.8s cubic-bezier(0.4, 0, 0.6, 1) infinite',
        'wave':          'wave 1s ease-in-out infinite',
        'glow-breathe':  'glow-breathe 2.4s ease-in-out infinite',
        'gradient-pan':  'gradient-pan 6s ease infinite',
        'checkmark':     'checkmark 0.45s cubic-bezier(0.22, 1, 0.36, 1) both',
        'pulse-slow':    'pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite',
      },
    },
  },
  plugins: [],
}
