# Nexus - realtime social game server
Projekat za predmet Napredne tehnike programiranja, Fakultet tehničkih nauka u Novom Sadu.

Za ocenu 10.

Autor: Vladislav Radović, SV 27/2021

## Opis problema

Izrada multiplayer video igara često zahteva od programera da implementira svoje rešenje centralnog servera koji će da skladišti stanje i podatke o korisnicima i igri, upravlja sobama i obavlja komunikaciju u realnom vremenu između klijenata.
Sa stanovišta programera koji razvija igricu, ovo predstavlja veliki dodatni posao koji je više fokusiran na oblast računarskih mreža, a ne na samu igru. Ovaj posao je većinski moguće generalizovati u vidu servisa koji bi predstavljao socijalni podsistem projekta.
Servis bi se sastojao od aplikativnog koda za biznis logiku, baze podataka kao skladište, i izloženim api-jem za HTTP/WebSocket komunikaciju sa klijentima. Za jednostavno podešavanje konfiguracije servisa, dostupan je i administratorski korisniči interfejs u vidu veb aplikacije.

Primeri postojećih rešenja:
- SmartFoxServer (https://www.smartfoxserver.com/)
- Nakama (https://heroiclabs.com/nakama/)

## Tehnologije

- Core aplikacija, backend - Rust
- Skladište - SQL baza (Postgres), Redis
- Message broker - NATS JetStream
- Klijentska aplikacija - Angular

## Funkcionalni zahtevi

### Core aplikacija
- **Upravljanje korisnicima**
  - Registracija i autentifikacija korisnika
  - Korisnički profili
  - Sistem prijatelja i blokiranje korisnika

- **Sistem soba i mečeva**
  - Automatsko pronalaženje protivnika **(Matchmaking System)**
  - Potvrđivanje pronađenog meča
  - Upravljanje kanalima za komunikaciju u toku igre

- **Chat sistem**
  - Chat u okviru meča
  - Istorija poruka
  - Osnovna moderacija sadržaja

- **Real-time komunikacija**
  - WebSocket konekcije
  - Sinhronizacija stanja igre
  - Broadcast događaja
  - Serverska validacija poteza kroz skriptu igre

### Administrativni panel
- **Web interfejs**
  - Pregled aktivnih korisnika
  - Pregled chat poruka
  - Upravljanje matchmaking pravilima

## Arhitektura

Projekat je organizovan kao skup manjih Rust servisa. `auth-service` upravlja nalozima i tokenima, `social-service` profilima, prijateljima, blokiranjem i chat-om, `matchmaking-service` redovima za pronalaženje mečeva, `realtime-service` WebSocket konekcijama i kanalima, a `game-service` pravilima igre kroz Rhai scripting jezik.

Servisi međusobno komuniciraju preko NATS JetStream poruka. PostgreSQL se koristi za trajne podatke, Redis za privremeno matchmaking stanje, a Angular aplikacija predstavlja klijentsku aplikaciju za registraciju, lobby, igru, prijatelje, chat i administraciju.
