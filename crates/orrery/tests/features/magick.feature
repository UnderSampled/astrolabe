@magick @functional
Feature: The selected Fire spell in the manor
  Background:
    Given Hype is ready to move in a clear part of the manor

  Scenario: Casting the selected Fire spell once
    Given Hype is grounded away from the manor save point
    When the player selects the sword and casts Fire once
    Then Hype enters the Fire cast and spends ten magic points
    And Hype reaches the Fire follow-through action
