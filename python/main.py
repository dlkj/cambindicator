import json
import time

import network
import rp2
import ubinascii
import urequests
import ntptime
import machine
from machine import Pin
from neopixel import NeoPixel

from icalendar import parse
from bindicator import get_bins_for_date


def main():

    pin = Pin(0, Pin.OUT)
    np = NeoPixel(pin, 16)
    try:
        inner_main(np)
    except Exception as e:
        # red - full error
        print(f"Uncaught error {e}")
        np.fill((255, 0, 0))
        np.write()
        time.sleep(1)
    machine.reset()


def get_config():
    rp2.country('GB')
    try:
        config_file = open('config.json', 'r')
        config = json.loads(config_file.read())
    finally:
        config_file.close()
    return config


def init_wifi(config):
    wlan = network.WLAN(network.STA_IF)
    wlan.active(True)
    wlan.connect(config.get('ssid'), config.get('password'))
    return wlan


def wait_for_wifi_connected(wlan, np):
    # Wait for wifi connection or failure
    max_wait = 30
    while max_wait > 0:
        if wlan.status() < 0 or wlan.status() >= 3:
            break
        max_wait -= 1
        print('waiting for connection...')
        time.sleep_ms(750)
        np.fill((0, 0, 0))
        np.write

    # Handle connection error
    if wlan.status() != 3:
        raise RuntimeError(
            f"network connection failed. status: {wlan.status()}")


def inner_main(np):
    led = machine.Pin("LED", machine.Pin.OUT)

    # blue - config and init
    np.fill((0, 0, 32))
    np.write()

    config = get_config()
    wlan = init_wifi(config)

    while wlan.status() == network.STAT_CONNECTING:
        print("Waiting for WiFi connection...")
        time.sleep_ms(250)
        np.fill((0, 0, 0))
        np.write()
        time.sleep_ms(750)
        np.fill((0, 0, 32))
        np.write()

    if wlan.status() != network.STAT_GOT_IP:
        raise Exception(f"WiFi connection failed. status: {wlan.status()}")

    print("WiFi connected")

    # network info
    print('ip: ' + wlan.ifconfig()[0])
    mac = ubinascii.hexlify(network.WLAN().config('mac'), ':').decode()
    print(f"mac: {mac}")

    # purple - connected
    np.fill((64, 0, 32))
    np.write()

    # init the RTC
    rtc = machine.RTC()
    time.sleep(1)
    get_ntp()
    print(f"RTC time: {rtc.datetime()}")

    np.fill((0, 0, 0))
    np.write()

    bins = {}
    seconds_since_calendar_update = 3600
    while True:
        seconds_since_calendar_update += 1
        if seconds_since_calendar_update > 3600:
            print("Fetching bin calendar...")
            get_ntp()

            print(f"RTC time: {rtc.datetime()}")
            calendar_events = get_events()
            seconds_since_calendar_update = 0
            print("Calendar fetched")
            bins = get_bins_for_date(calendar_events, tomorrow(rtc.datetime()))

        (_, _, _, _, hour, _, _, _) = rtc.datetime()

        if 17 <= hour <= 22:
            if bins == {"GREEN"}:
                np.fill((0, 255, 0))
            elif bins == {"BLUE"}:
                np.fill((0, 0, 255))
            elif bins == {"BLACK"}:
                np.fill((255, 255, 255))
            elif bins == {"GREEN", "BLUE"}:
                for i in range(0, 8):
                    np[i] = (0, 0, 128)
                for i in range(8, 16):
                    np[i] = (0, 255, 0)
            else:
                np.fill((0, 0, 0))
        else:
            np.fill((0, 0, 0))
        np.write()

        led.toggle()
        time.sleep(1)


def tomorrow(date):

    days_in_month = {
        1: 31, 2: 28, 3: 31, 4: 30, 5: 31, 6: 30, 7: 31, 8: 31, 9: 30, 10: 31, 11: 30, 12: 31
    }

    (year, month, day, hour, min, sec, ms, ns) = date

    if year % 4 == 0 and (year % 100 != 100 or year % 400 == 0):
        days_in_month[2] = 29

    day += 1

    if day > days_in_month[month]:
        day = 0
        month += 1

    return (year, month, day, hour, min, sec, ms, ns)


def get_events():
    try:
        response = urequests.get(
            "https://servicelayer3c.azure-api.net/wastecalendar/calendar/ical/200004185983")
        (_, events) = parse(response.text.splitlines())
    finally:
        response.close()
    return events


def get_ntp():
    for _ in range(0, 30):
        try:
            ntptime.settime()
            return
        except Exception as e:
            print(f"Failed to set RTC {e}")
            time.sleep_ms(1000)
    raise Exception("Failed to set RTC")


if __name__ == "__main__":
    main()
