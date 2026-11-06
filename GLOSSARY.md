# DynoWatt

Local-first analytics for cyclists: importing recorded rides, inspecting their telemetry, modelling the athlete's power-duration profile, and planning structured sessions to ride from a Bike Computer.

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
_Avoid_: Ride, session, training, Workout (that is planned, not recorded)

**Indoor Activity**:
An Activity ridden on a home trainer, as opposed to an outdoor ride on the road or trails.
_Avoid_: Virtual ride, trainer session, Zwift

**Activity Summary**:
The headline figures of an Activity: local start time, Moving Time, distance, elevation gain, Average Power and average heart rate.
_Avoid_: Stats, overview, totals

**Average Power**:
Mean power over the Moving Time, counting zero-watt coasting but excluding Gaps.
_Avoid_: Mean power, avg watts

**Bike Computer**:
The device, on the handlebars or the wrist, that records Activities and guides the Athlete through Workouts.
_Avoid_: Device, head unit, GPS unit, Garmin

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

**FTP** (Functional Threshold Power):
The highest power the Athlete can hold for about an hour; the reference Power Targets are expressed against.
_Avoid_: Threshold power, CP (a different, model-based estimate)

**W'** (W prime):
The finite amount of work, in joules, the Athlete can perform above CP before exhaustion.
_Avoid_: Anaerobic capacity, battery

**W'bal** (W' balance):
How much of W' remains at each instant of an Activity, depleting above CP and recovering below it.
_Avoid_: W' remaining, matches

### Workouts

**Workout**:
A named, planned cycling session, described as an ordered sequence of Steps and Repeats, for the Athlete to ride later from their Bike Computer. A Workout is planned; an Activity is recorded.
_Avoid_: Training plan, session, structured workout, interval session

**Step**:
One part of a Workout with a fixed duration, a Step Kind and an optional Power Target.
_Avoid_: Interval, block, segment, phase

**Step Kind**:
The role a Step plays in a Workout: Warm-up, Work, Recovery or Cool-down.
_Avoid_: Intensity, type, zone

**Repeat**:
A sequence of Steps ridden a given number of times in a row, such as 3 × (5 min Work, 5 min Recovery). A Repeat holds only Steps, never another Repeat.
_Avoid_: Set, loop, block

**Power Target**:
The range of power, from a lower to a strictly higher percentage of the FTP set on the Bike Computer, that the Athlete aims to hold during a Step. A Step without a Power Target is ridden freely.
_Avoid_: Zone, intensity, goal

**Workout File**:
The `.FIT` file DynoWatt writes from a Workout for the Athlete to copy onto their Bike Computer. DynoWatt writes it rather than imports it, and keeps no trace of it once written.
_Avoid_: Export, plan file, Source File
