# DynoWatt

Local-first analytics for cyclists: importing recorded rides, inspecting their telemetry, and modelling the athlete's power-duration profile.

## Language

### People

**Athlete**:
The cyclist whose activities are analysed. One installation tracks a single Athlete for now.
_Avoid_: User, rider, profile

**Athlete Parameters**:
A dated set of physiological values (CP, W', FTP, body weight, max heart rate) describing the Athlete from a given date onward. An Activity is always analysed with the Athlete Parameters in effect on its date.
_Avoid_: Settings, profile, zones

### Activities

**Activity**:
One recorded cycling session, imported from exactly one Source File.
_Avoid_: Ride, workout, session, training

**Indoor Activity**:
An Activity ridden on a home trainer, as opposed to an outdoor ride on the road or trails.
_Avoid_: Virtual ride, trainer session, Zwift

**Activity Summary**:
The headline figures of an Activity: local start time, Moving Time, distance, elevation gain, Average Power and average heart rate.
_Avoid_: Stats, overview, totals

**Average Power**:
Mean power over the Moving Time, counting zero-watt coasting but excluding Gaps.
_Avoid_: Mean power, avg watts

**Source File**:
The original `.FIT` file an Activity was imported from, kept unmodified as the reference copy of its data.
_Avoid_: Upload, raw file

**Duplicate**:
A Source File recognised as one already in the Library, even if its bytes differ (for example after re-export from another platform).
_Avoid_: Copy, re-import

**Import Report**:
The outcome of one import run, telling for each Source File whether it was imported, a Duplicate, rejected as not cycling, or rejected as unreadable.
_Avoid_: Import log, sync result

**Library**:
The collection of Source Files DynoWatt has imported and keeps on the Athlete's machine.
_Avoid_: Archive, storage, vault

**Sample**:
The values (power, heart rate, cadence, altitude, …) for one second of an Activity's timeline.
_Avoid_: Record, point, datapoint, tick

**Position**:
Where the Athlete was at one second of an Activity, as latitude and longitude. Like every value of a Sample, it can be missing.
_Avoid_: GPS point, coordinate, location, lat/lng

**Gap**:
A stretch of an Activity's timeline with no Sample, typically an auto-pause or a recording dropout. A Gap is absence of data, never zero power.
_Avoid_: Pause, stop, zero

**Elapsed Time**:
The time from an Activity's first Sample to its last, Gaps included.
_Avoid_: Total time, duration

**Moving Time**:
Elapsed Time minus all Gaps; the duration shown for an Activity by default.
_Avoid_: Ride time, active time, duration

**Track**:
The line an Activity's Positions draw on a map, in timeline order.
_Avoid_: Route (a planned course), trace, path, GPS track

### Physiology

**MMP** (Mean Maximal Power):
The highest average power the Athlete sustained over a given duration, within one Activity or across many.
_Avoid_: Best effort, peak power

**PDC** (Power Duration Curve):
The MMP plotted against every duration, describing how power falls as effort length increases.
_Avoid_: Power curve, CP curve

**CP** (Critical Power):
The highest power the Athlete can sustain without drawing down W', as estimated from the PDC.
_Avoid_: Threshold (that is FTP)

**W'** (W prime):
The finite amount of work, in joules, the Athlete can perform above CP before exhaustion.
_Avoid_: Anaerobic capacity, battery

**W'bal** (W' balance):
How much of W' remains at each instant of an Activity, depleting above CP and recovering below it.
_Avoid_: W' remaining, matches
