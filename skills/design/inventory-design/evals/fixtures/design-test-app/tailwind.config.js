/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ['./src/**/*.{js,jsx}'],
  theme: {
    extend: {
      colors: {
        primary: '#2563eb',
        'primary-muted': '#93c5fd',
        surface: '#f8fafc',
        danger: '#dc2626',
      },
    },
  },
}
