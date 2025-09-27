# Nexus - realtime social game server
Projekat za predmet Napredne tehnike programiranja, Fakultet tehničkih nauka u Novom Sadu.

Za ocenu 10.

Autor: Vladislav Radović, SV 27/2021

## Opis problema

Izrada multiplayer video igara često zahteva od programera da implementira svoje rešenje centralnog servera koji će da skladišti stanje i podatke o korisnicima i igri, upravlja sobama i obavlja komunikaciju u realnom vremenu između klijenata.
Sa stanovišta programera koji razvija igricu, ovo predstavlja veliki dodatni posao koji je više fokusiran na oblast računarskih mreža, a ne na samu igru. Ovaj posao je većinski moguće generalizovati u vidu servisa koji bi predstavljao socijalni podsistem projekta.
Servis bi se sastojao od aplikativnog koda za biznis logiku, baze podataka kao skladište, i izloženim api-jem za HTTP/WebSocket komunikaciju sa klijentima. Za jednostavno podešavanje konfiguracije servisa, dostupan je i administratorski korisniči interfejs u vidu veb aplikacije.

## Tehnologije

- Core aplikacija, backend - Rust
- Skladište - SQL baza (Postgres)
- Administratorska klijentska aplikacija - React

## Funkcionalni zahtevi

### Core aplikacija
- **Upravljanje korisnicima**
  - Registracija i autentifikacija korisnika
  - Korisnički profili i podešavanja
  - Sistem prijatelja i blokiranje korisnika

- **Sistem soba**
  - Kreiranje javnih i privatnih soba
  - Automatsko pronalaženje soba **(Matchmaking System)**
  - Upravljanje kapacitetom soba

- **Chat sistem**
  - Globalni, grupni i privatni chat
  - Istorija poruka
  - Moderacija sadržaja

- **Real-time komunikacija**
  - WebSocket konekcije
  - Sinhronizacija stanja igre
  - Broadcast događaja

### Administrativni panel
- **Web interfejs**
  - Pregled aktivnih korisnika i soba
  - Moderacija chat-a
  - Statistike servera
  - Upravljanje korisničkim nalozima

## Proširenje za diplomski rad
Postoje dve oblasti za koje smatram da bi značajno poboljšale funkcionalnosti i performanse ovog projekta:

1. **Integrisani skripting jezik** - pored osnovnih funkcionalnosti, ubacivanjem integrisanog skripting jezika u core aplikaciju bi omogućilo korisnicima da definišu dodatna pravila i logiku koja bi se izvršavala u mnogim delovima aplikacije (hooks).
Takođe postoji i mogućnost pisanja pravila igre, tako da bi za neke jednostavnije turn-based igre, kao što je šah, bilo moguće i definisati celu logiku igrice kroz ova pravila (u teoriji je moguće za svaku igricu ali u realtime igrama gde je od velikog značaja brzina bi dedicated serveri bila bolja opcija).
Rešenje za ovo bi moglo biti pisanje sopstvenog jednostavnog skripting jezika, ili upotrebom postojećeg (https://github.com/rhaiscript/rhai).
2. **Horizontalno skaliranje servera i međusobna komunikacija** - ukoliko bi došlo do previsokog broja korisnika i soba, bilo bi zgodno implementirati rešenje za podizanje više instanci ovog servisa (na istom ili različitim serverima) i mehanizam komunikacije između njih (preko message brokera). Ovo bi moglo značajno da rastereti sistem, dok omogućuje visok broj klijenata.
