import { describe, expect, test } from 'bun:test';
import { defaultLocale, isSupportedLocale, localize, supportedLocales, translate } from './i18n';

describe('i18n', () => {
  test('declares Portuguese, English, Spanish, German and Russian as supported locales', () => {
    expect(defaultLocale).toBe('pt-BR');
    expect(supportedLocales.map((locale) => locale.code)).toEqual(['pt-BR', 'en', 'es', 'de', 'ru']);
  });

  test('rejects unsupported locale values', () => {
    expect(isSupportedLocale('pt-BR')).toBe(true);
    expect(isSupportedLocale('fr')).toBe(false);
    expect(isSupportedLocale('')).toBe(false);
  });

  test('translates known keys in each supported locale', () => {
    expect(translate('pt-BR', 'nav.overview')).toBe('Visao geral');
    expect(translate('en', 'nav.overview')).toBe('Overview');
    expect(translate('es', 'nav.overview')).toBe('Resumen');
    expect(translate('de', 'nav.overview')).toBe('Ubersicht');
    expect(translate('ru', 'nav.overview')).toBe('Обзор');
  });

  test('falls back to the default locale for invalid locale input', () => {
    expect(localize('fr').text('nav.rules')).toBe('Regras');
  });

  test('interpolates parameters without evaluating markup', () => {
    const message = translate('en', 'summary.description', {
      services: '<6>',
      rules: '3',
      certificates: '2'
    });

    expect(message).toBe('<6> services, 3 active security rules, 2 certificate workflows.');
  });
});
